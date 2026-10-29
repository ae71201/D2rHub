//! Event-triggered souvenirs, never evidence used to accept or reject a drop.
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::Manager;

pub(crate) fn directory(root: &str) -> PathBuf {
    Path::new(root).join("stateData").join("drop-screenshots")
}

fn component(value: &str) -> String {
    let value: String = value
        .chars()
        .take(60)
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let value = value.trim().trim_matches('.');
    if value.is_empty() {
        "未知".into()
    } else {
        value.into()
    }
}

fn relative_path(
    observed_at: &str,
    scene: &str,
    name: &str,
    rune: u32,
    id: i64,
) -> Result<String, String> {
    let time = chrono::DateTime::parse_from_rfc3339(observed_at).map_err(|e| e.to_string())?;
    let identity = if id > 0 {
        id.to_string()
    } else {
        uuid::Uuid::new_v4().to_string()
    };
    Ok(format!(
        "drop-screenshots/{}/{}_{}_{}#{}_drop-{}.png",
        time.format("%Y-%m-%d"),
        time.format("%Y-%m-%d_%H-%M-%S%.3f%z"),
        component(scene),
        component(name),
        rune,
        identity
    ))
}

fn save(root: &str, relative: &str, image: image::RgbImage) -> Result<(), String> {
    let path = Path::new(root).join("stateData").join(relative);
    std::fs::create_dir_all(path.parent().ok_or("截图路径无效")?).map_err(|e| e.to_string())?;
    // Never overwrite an earlier event, even if the same job is submitted twice.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let mut writer = std::io::BufWriter::new(file);
    let result = image::DynamicImage::ImageRgb8(image)
        .write_to(&mut writer, image::ImageFormat::Png)
        .map_err(|e| e.to_string())
        .and_then(|()| writer.flush().map_err(|e| e.to_string()));
    drop(writer);
    if result.is_err() {
        let _ = std::fs::remove_file(&path);
    }
    result
}

pub(crate) fn schedule(
    app: &tauri::AppHandle,
    pid: u32,
    id: i64,
    rune: u32,
    observed_at: &str,
    scene: &str,
    name: &str,
) {
    if !(24..=33).contains(&rune) {
        return;
    }
    let root = app
        .state::<crate::state::SharedState>()
        .app_data_dir
        .clone();
    let relative = match relative_path(observed_at, scene, name, rune, id) {
        Ok(value) => value,
        Err(error) => {
            log_failure(&error);
            return;
        }
    };
    // Capture and PNG encoding must not stall audio processing. No OCR or
    // recognition service is started for this request.
    if let Err(error) = std::thread::Builder::new()
        .name("drop-screenshot".into())
        .spawn(move || {
            let result = (|| {
                let captured = crate::vision::capture::capture(pid, None)?;
                save(&root, &relative, captured.image)?;
                if id > 0 {
                    crate::stats::attach_drop_screenshot(&root, id, &relative)?;
                }
                crate::logger::log_msg(
                    "INFO",
                    "DropScreenshot",
                    &format!("掉落截图已保存：{relative}"),
                );
                Ok::<_, String>(())
            })();
            if let Err(error) = result {
                log_failure(&error);
            }
        })
    {
        log_failure(&error.to_string());
    }
}

fn log_failure(error: &str) {
    crate::logger::log_msg(
        "WARN",
        "DropScreenshot",
        &format!("掉落截图未完成，声纹记录保留：{error}"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filename_preserves_event_time_and_identity_without_windows_invalid_characters() {
        let path =
            relative_path("2026-10-29T10:54:24.123+08:00", "混沌魔殿", "贝", 30, 123).unwrap();
        assert_eq!(
            path,
            "drop-screenshots/2026-10-29/2026-10-29_10-54-24.123+0800_混沌魔殿_贝#30_drop-123.png"
        );
        let unsafe_name = relative_path(
            "2026-10-29T10:54:24+08:00",
            "../场景:甲",
            "a/b\\c*",
            24,
            124,
        )
        .unwrap();
        assert_eq!(unsafe_name.split('/').count(), 3);
        assert!(!unsafe_name.contains(':'));
        assert_ne!(
            path,
            relative_path("2026-10-29T10:54:24.123+08:00", "混沌魔殿", "贝", 30, 124).unwrap()
        );
    }
    #[test]
    fn saves_png_and_does_not_overwrite_existing_screenshot() {
        let root = std::env::temp_dir().join(format!("drop-capture-{}", uuid::Uuid::new_v4()));
        let relative = relative_path("2026-10-29T10:54:24+08:00", "场景", "伊斯特", 24, 1).unwrap();
        let image = image::RgbImage::from_pixel(8, 6, image::Rgb([20, 40, 60]));
        save(root.to_str().unwrap(), &relative, image.clone()).unwrap();
        let path = root.join("stateData").join(&relative);
        assert_eq!(image::open(&path).unwrap().to_rgb8(), image);
        assert!(save(root.to_str().unwrap(), &relative, image).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
