//! Cheap red-text projection gate; OCR always receives original RGB pixels.
use super::{processing, Candidate};
use image::{imageops, RgbImage};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

pub fn red([r, g, b]: [u8; 3]) -> bool {
    let (r, g, b) = (r as u32, g as u32, b as u32);
    r >= 65 && r >= g * 3 / 2 && r >= b * 3 / 2 && r - g >= 35 && r - b >= 30
}
#[derive(Debug, serde::Serialize)]
pub struct Feature {
    pub present: bool,
    pub pixels: usize,
    pub strokes: usize,
    pub bounds: Option<(u32, u32, u32, u32)>,
}
pub fn feature(image: &RgbImage) -> Feature {
    feature_color(image, red)
}
fn feature_color(image: &RgbImage, color: impl Fn([u8; 3]) -> bool) -> Feature {
    let (w, h) = image.dimensions();
    let mut columns = vec![0usize; w as usize];
    let mut rows = vec![0usize; h as usize];
    let mut pixels = 0;
    for (x, y, p) in image.enumerate_pixels() {
        if color(p.0) {
            columns[x as usize] += 1;
            rows[y as usize] += 1;
            pixels += 1;
        }
    }
    let min_column = (h as usize / 30).clamp(2, 8);
    let xs: Vec<_> = columns
        .iter()
        .enumerate()
        .filter(|(_, n)| **n >= min_column)
        .map(|(x, _)| x as u32)
        .collect();
    let min_row = (w as usize / 150).max(3);
    let ys: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(_, n)| **n >= min_row)
        .map(|(y, _)| y as u32)
        .collect();
    let bounds = xs
        .first()
        .zip(xs.last())
        .zip(ys.first().zip(ys.last()))
        .map(|((&x0, &x1), (&y0, &y1))| (x0, y0, x1 + 1, y1 + 1));
    let strokes = columns
        .iter()
        .fold((0, false), |(n, on), value| {
            let next = *value >= min_column;
            (n + usize::from(next && !on), next)
        })
        .0;
    let present = bounds.is_some_and(|(x0, y0, x1, y1)| {
        let bw = x1 - x0;
        let bh = y1 - y0;
        let density = pixels as f32 / (bw * bh).max(1) as f32;
        bh >= 9
            && bh * 12 >= h
            && bw >= bh * 3
            && bw <= bh * 45
            && strokes >= 3
            && (0.045..=0.72).contains(&density)
    });
    Feature {
        present,
        pixels,
        strokes,
        bounds,
    }
}
fn purple([r, g, b]: [u8; 3]) -> bool {
    r >= 70
        && b >= 100
        && b as u32 * 10 >= r as u32 * 11
        && b as u32 > g as u32 * 3 / 2
        && r as u32 > g as u32 * 6 / 5
}
pub fn tz_crop(image: &RgbImage, entry: &Feature) -> Option<RgbImage> {
    let (_, _, _, bottom) = entry.bounds.filter(|_| entry.present)?;
    if bottom >= image.height() {
        return None;
    }
    let below =
        imageops::crop_imm(image, 0, bottom, image.width(), image.height() - bottom).to_image();
    let feature = feature_color(&below, purple);
    crop(&below, &feature)
}
fn terror_text(text: &str) -> bool {
    [
        "恐怖区域",
        "恐惧区域",
        "恐怖區域",
        "恐懼區域",
        "Terror Zone",
    ]
    .iter()
    .any(|v| processing::normalize(text) == processing::normalize(v))
}
pub fn crop(image: &RgbImage, feature: &Feature) -> Option<RgbImage> {
    let (x0, y0, x1, y1) = feature.bounds.filter(|_| feature.present)?;
    let pad = ((y1 - y0) / 10).clamp(2, 8);
    let x = x0.saturating_sub(pad);
    let y = y0.saturating_sub(pad);
    Some(
        imageops::crop_imm(
            image,
            x,
            y,
            (x1 + pad).min(image.width()) - x,
            (y1 + pad).min(image.height()) - y,
        )
        .to_image(),
    )
}
pub fn resolve(text: &str, candidates: &[Candidate]) -> Option<Candidate> {
    let text = text.trim();
    // A map-entry announcement must contain its explicit prefix. Never accept
    // arbitrary red chat, health text, or a best-effort nearest catalog name.
    let body = text
        .strip_prefix("进入")
        .or_else(|| text.strip_prefix("進入"))
        .or_else(|| text.strip_prefix("Entering "))?;
    let mut found = candidates.iter().filter(|candidate| {
        candidate
            .aliases
            .iter()
            .any(|alias| processing::normalize(alias) == processing::normalize(body))
    });
    let candidate = found.next()?.clone();
    found.next().is_none().then_some(candidate)
}
#[derive(Default)]
pub struct Gate {
    signature: Option<[u8; 32]>,
    result: Option<Candidate>,
    retry: Option<Instant>,
    pub last_skip: Option<&'static str>,
    entry_name: Option<String>,
    entry_id: Option<u64>,
    tz_seen: bool,
}
impl Gate {
    pub fn absent(&mut self) {
        self.signature = None;
        self.result = None;
        self.entry_name = None;
        self.entry_id = None;
        self.tz_seen = false;
    }
    pub fn read(
        &mut self,
        image: &RgbImage,
        now: Instant,
        candidates: &[Candidate],
        tz: Option<&RgbImage>,
        mut recognize: impl FnMut(&RgbImage) -> Result<(String, f32), String>,
    ) -> Result<Option<Candidate>, String> {
        self.last_skip = None;
        let mut hash = Sha256::new();
        hash.update(image.width().to_le_bytes());
        hash.update(image.height().to_le_bytes());
        // Background animation must not invalidate an otherwise identical glyph mask.
        for p in image.pixels() {
            hash.update([u8::from(red(p.0))]);
        }
        hash.update([u8::from(tz.is_some())]);
        if let Some(tz) = tz {
            hash.update(tz.width().to_le_bytes());
            hash.update(tz.height().to_le_bytes());
            for p in tz.pixels() {
                hash.update([u8::from(purple(p.0))]);
            }
        }
        let signature = hash.finalize().into();
        if self.signature == Some(signature)
            && (self.result.is_some() || self.retry.is_some_and(|until| until > now))
        {
            self.last_skip = Some(if self.result.is_some() {
                "unchanged_red_mask"
            } else {
                "retry_cooldown"
            });
            return Ok(self.result.clone());
        }
        self.signature = Some(signature);
        self.result = None;
        self.retry = Some(now + Duration::from_secs(2));
        let (text, score) = recognize(image)?;
        if score >= 0.90 {
            self.result = resolve(&text, candidates);
            if let Some(result) = self.result.as_mut() {
                result.confidence = score;
            }
            if self.result.is_some() {
                let result = self.result.as_mut().unwrap();
                if self.entry_name.as_deref() != Some(result.name.as_str()) {
                    static NEXT: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(1);
                    self.entry_name = Some(result.name.clone());
                    self.entry_id = Some(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
                    self.tz_seen = false;
                }
                result.announcement = self.entry_id;
                result.tz = self.tz_seen;
                if !self.tz_seen {
                    if let Some(tz) = tz {
                        let (text, score) = recognize(tz)?;
                        if score < 0.90 || !terror_text(&text) {
                            self.result = None;
                        } else if let Some(result) = self.result.as_mut() {
                            self.tz_seen = true;
                            result.tz = true;
                            result.confidence = result.confidence.min(score);
                        }
                    }
                }
            }
        }
        Ok(self.result.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tz_is_bound_to_one_announcement_and_does_not_disappear_when_purple_fades() {
        let candidates = vec![Candidate {
            name: "洞穴".into(),
            aliases: vec!["洞穴".into()],
            ..Default::default()
        }];
        let red = RgbImage::from_pixel(100, 20, image::Rgb([160, 0, 0]));
        let purple = RgbImage::from_pixel(60, 20, image::Rgb([140, 40, 200]));
        let now = Instant::now();
        let mut gate = Gate::default();
        let normal = gate
            .read(&red, now, &candidates, None, |_| {
                Ok(("进入洞穴".into(), 0.99))
            })
            .unwrap()
            .unwrap();
        let mut calls = 0;
        let terror = gate
            .read(&red, now, &candidates, Some(&purple), |_| {
                calls += 1;
                Ok((
                    if calls == 1 {
                        "进入洞穴"
                    } else {
                        "恐惧区域"
                    }
                    .into(),
                    0.99,
                ))
            })
            .unwrap()
            .unwrap();
        assert!(terror.tz);
        assert_eq!(normal.announcement, terror.announcement);
        let fading = gate
            .read(&red, now, &candidates, None, |_| {
                Ok(("进入洞穴".into(), 0.99))
            })
            .unwrap()
            .unwrap();
        assert!(fading.tz);
        gate.absent();
        let next = gate
            .read(&red, now, &candidates, None, |_| {
                Ok(("进入洞穴".into(), 0.99))
            })
            .unwrap()
            .unwrap();
        assert!(!next.tz);
        assert_ne!(next.announcement, normal.announcement);
    }
    #[test]
    fn retries_failed_text_but_reuses_success_despite_background_animation() {
        let candidates = vec![Candidate {
            name: "洞穴".into(),
            name_en: "Cave".into(),
            aliases: vec!["洞穴".into()],
            ..Default::default()
        }];
        let mut gate = Gate::default();
        let now = Instant::now();
        let mut image = RgbImage::new(200, 25);
        image.put_pixel(10, 10, image::Rgb([160, 0, 0]));
        assert!(gate
            .read(&image, now, &candidates, None, |_| Ok((
                "进入洞穴".into(),
                0.89
            )))
            .unwrap()
            .is_none());
        assert!(gate
            .read(
                &image,
                now + Duration::from_secs(1),
                &candidates,
                None,
                |_| panic!("retry cooldown")
            )
            .unwrap()
            .is_none());
        assert!(gate
            .read(
                &image,
                now + Duration::from_secs(2),
                &candidates,
                None,
                |_| Ok(("进入洞穴".into(), 0.99))
            )
            .unwrap()
            .is_some());
        image.put_pixel(20, 10, image::Rgb([50, 50, 50]));
        assert!(gate
            .read(
                &image,
                now + Duration::from_secs(3),
                &candidates,
                None,
                |_| panic!("background changes are not glyph changes")
            )
            .unwrap()
            .is_some());
        image.put_pixel(30, 10, image::Rgb([160, 0, 0]));
        assert!(gate
            .read(
                &image,
                now + Duration::from_secs(3),
                &candidates,
                None,
                |_| Ok(("进入地穴".into(), 0.99))
            )
            .unwrap()
            .is_none());
    }
    #[test]
    fn rejects_solid_red_regions_and_requires_entry_prefix_and_unique_name() {
        assert!(!feature(&RgbImage::from_pixel(300, 35, image::Rgb([180, 0, 0]))).present);
        let candidates = vec![Candidate {
            name: "世界之石要塞二层".into(),
            name_en: "Worldstone Keep Level 2".into(),
            aliases: vec!["世界之石要塞二层".into()],
            ..Default::default()
        }];
        assert!(resolve("进入世界之石要塞二层", &candidates).is_some());
        assert!(resolve("世界之石要塞二层", &candidates).is_none());
        assert!(resolve("进入世界之石要塞三层", &candidates).is_none());
    }
}
