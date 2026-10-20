use super::resources::Installed;
use image::{
    imageops::{resize, FilterType},
    RgbImage,
};
use ort::{session::Session, value::Tensor};
pub struct Ocr {
    session: Session,
    dictionary: Vec<String>,
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
impl Ocr {
    pub fn new(files: &Installed) -> Result<Self, String> {
        static LOADED_RUNTIME: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
        let loaded = LOADED_RUNTIME.get_or_init(|| files.runtime.clone());
        if loaded != &files.runtime {
            return Err("推理运行库已更新，请重启软件以使用新版本".into());
        }
        ort::init_from(files.runtime.to_string_lossy())
            .commit()
            .map_err(err)?;
        let session = Session::builder()
            .map_err(err)?
            .with_intra_threads(2)
            .map_err(err)?
            .with_inter_threads(1)
            .map_err(err)?
            .commit_from_file(&files.model)
            .map_err(err)?;
        let dictionary = session
            .metadata()
            .map_err(err)?
            .custom("character")
            .map_err(err)?
            .ok_or("OCR 模型缺少内置字典")?;
        let mut dictionary: Vec<String> = dictionary.lines().map(str::to_owned).collect();
        dictionary.insert(0, String::new());
        dictionary.push(" ".into());
        Ok(Self {
            session,
            dictionary,
        })
    }
    pub fn recognize(&mut self, image: &RgbImage) -> Result<(String, f32), String> {
        let width = ((image.width() as f32 / image.height().max(1) as f32 * 48.0).ceil() as u32)
            .clamp(16, 1536);
        let resized = resize(image, width, 48, FilterType::Triangle);
        let mut input = vec![0f32; 3 * 48 * width as usize];
        for (x, y, p) in resized.enumerate_pixels() {
            for c in 0..3 {
                // RapidOCR uses BGR input normalized to [-1,1].
                input[c * 48 * width as usize + y as usize * width as usize + x as usize] =
                    p.0[2 - c] as f32 / 127.5 - 1.0;
            }
        }
        let tensor = Tensor::from_array(([1usize, 3, 48, width as usize], input)).map_err(err)?;
        let outputs = self.session.run(ort::inputs![tensor]).map_err(err)?;
        let (shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(err)?;
        if shape.len() != 3 || shape[0] != 1 || shape[2] as usize != self.dictionary.len() {
            return Err(format!(
                "OCR 输出与字典不匹配: {shape:?} / {}",
                self.dictionary.len()
            ));
        }
        decode(data, &self.dictionary)
    }
}
fn decode(data: &[f32], dict: &[String]) -> Result<(String, f32), String> {
    let mut text = String::new();
    let mut previous = 0;
    let mut sum = 0.0;
    let mut count = 0;
    for row in data.chunks_exact(dict.len()) {
        let (id, score) = row
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .ok_or("空 OCR 输出")?;
        if !score.is_finite() {
            return Err("OCR 输出无效".into());
        }
        if id != 0 && id != previous {
            text.push_str(&dict[id]);
            sum += *score;
            count += 1;
        }
        previous = id;
    }
    Ok((text, if count > 0 { sum / count as f32 } else { 0.0 }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ctc_keeps_repeats_separated_by_blank() {
        let dict = vec!["".into(), "+".into()];
        assert_eq!(
            decode(&[0., 1., 0., 1., 1., 0., 0., 1.], &dict).unwrap().0,
            "++"
        );
    }
    #[test]
    #[ignore = "Read-only drop pipeline probe; set D2RHUB_DROP_SAMPLE to a screenshot PNG"]
    fn drop_screenshot_probe() {
        use super::super::{drop_match, labels};
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dev/vision-resources");
        let stop = std::sync::atomic::AtomicBool::new(false);
        let files = tauri::async_runtime::block_on(super::super::resources::ensure(
            &root.join("cache"),
            &stop,
            |status| println!("{status}"),
        ))
        .unwrap();
        let mut ocr = Ocr::new(&files).unwrap();
        let sample_path = std::env::var("D2RHUB_DROP_SAMPLE").unwrap();
        let image = image::open(&sample_path).unwrap().to_rgb8();
        let output = root
            .join("drop-label-probe")
            .join(std::path::Path::new(&sample_path).file_stem().unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let started = std::time::Instant::now();
        let lines = labels::detect(&image);
        let segmentation_ms = started.elapsed().as_secs_f64() * 1000.0;
        let mut records = Vec::new();
        for (index, label) in lines.iter().enumerate() {
            let line = &label.name;
            line.save(output.join(format!("line-{index:02}.png")))
                .unwrap();
            let started = std::time::Instant::now();
            let original = ocr.recognize(line).unwrap();
            let selected = &original;
            let matches: Vec<_> = (1..=33)
                .filter(|number| {
                    selected.1 >= 0.90
                        && drop_match::matches(
                            &selected.0,
                            &crate::rune_data::visual_rune_aliases(*number),
                        )
                })
                .collect();
            let record = serde_json::json!({"index":index,"rect":label.rect,"background":label.background,"score":label.score,"width":line.width(),"height":line.height(),"original":original,"selected":selected,"matched_runes":matches,"ocr_ms":started.elapsed().as_secs_f64()*1000.0});
            println!("{record}");
            records.push(record);
        }
        let report = serde_json::json!({"image_width":image.width(),"image_height":image.height(),"segmentation_ms":segmentation_ms,"lines":records});
        if let Ok(expected) = std::env::var("D2RHUB_DROP_EXPECT") {
            for number in expected.split(',').map(|v| v.parse::<u32>().unwrap()) {
                assert!(
                    records.iter().any(|record| record["matched_runes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v == number)),
                    "missing rune {number}: {report}"
                );
            }
        }
        std::fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        println!("Report: {}", output.join("report.json").display());
    }
    #[test]
    #[ignore = "Read-only red-entry pipeline probe; set D2RHUB_AREA_SAMPLE to a cropped announcement PNG"]
    fn area_screenshot_probe() {
        use super::super::{area, Candidate};
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dev/vision-resources");
        let stop = std::sync::atomic::AtomicBool::new(false);
        let files = tauri::async_runtime::block_on(super::super::resources::ensure(
            &root.join("cache"),
            &stop,
            |s| println!("{s}"),
        ))
        .unwrap();
        let mut ocr = Ocr::new(&files).unwrap();
        let image = image::open(std::env::var("D2RHUB_AREA_SAMPLE").unwrap())
            .unwrap()
            .to_rgb8();
        let started = std::time::Instant::now();
        let feature = area::feature(&image);
        println!(
            "Feature: {} in {:.3} ms",
            serde_json::to_string(&feature).unwrap(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        let crop = area::crop(&image, &feature).expect("red text gate should pass");
        crop.save(root.join("area-original-crop.png")).unwrap();
        let expected =
            std::env::var("D2RHUB_AREA_EXPECT").unwrap_or_else(|_| "世界之石要塞二层".into());
        let candidates = vec![Candidate {
            name: expected.clone(),
            name_en: expected.clone(),
            aliases: vec![expected.clone()],
            ..Default::default()
        }];
        let tz = area::tz_crop(&image, &feature);
        if let Some(tz) = &tz {
            tz.save(root.join("area-tz-original-crop.png")).unwrap();
        }
        let mut gate = area::Gate::default();
        let found = gate
            .read(
                &crop,
                std::time::Instant::now(),
                &candidates,
                tz.as_ref(),
                |line| {
                    let result = ocr.recognize(line)?;
                    println!("OCR: {result:?}");
                    Ok(result)
                },
            )
            .unwrap();
        let found = found.unwrap();
        assert_eq!(found.name, expected);
        assert_eq!(
            found.tz,
            std::env::var("D2RHUB_AREA_TZ").is_ok_and(|v| v == "1")
        );
        gate.read(
            &crop,
            std::time::Instant::now(),
            &candidates,
            tz.as_ref(),
            |_| panic!("unchanged red text must not invoke OCR"),
        )
        .unwrap();
    }
    #[test]
    #[ignore = "Downloads optional assets and runs native ORT; set D2RHUB_VISION_SMOKE_IMAGE to a local PNG"]
    fn downloaded_model_smoke() {
        let root = std::env::var_os("D2RHUB_VISION_SMOKE_CACHE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../dev/vision-resources/cache")
            });
        let stop = std::sync::atomic::AtomicBool::new(false);
        let files =
            tauri::async_runtime::block_on(super::super::resources::ensure(&root, &stop, |s| {
                println!("{s}")
            }))
            .unwrap();
        let mut ocr = Ocr::new(&files).unwrap();
        let path = std::env::var("D2RHUB_VISION_SMOKE_IMAGE").unwrap();
        let image = image::open(path).unwrap().to_rgb8();
        let (text, score) = ocr.recognize(&image).unwrap();
        println!("OCR: {text} ({score})");
        assert!(text.contains("037"));
        assert!(score > 0.8);
    }
}
