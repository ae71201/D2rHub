//! Background scanlines locate rectangles. Projections only crop intact text.
use image::{imageops, RgbImage};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Background {
    Dark,
    Blue,
}
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug)]
pub struct Label {
    pub rect: Rect,
    pub name: RgbImage,
    pub background: Background,
    pub score: f32,
}
fn background([r, g, b]: [u8; 3], kind: Background) -> bool {
    match kind {
        Background::Dark => r.max(g).max(b) < 100,
        Background::Blue => {
            b > 65 && b as u32 > r as u32 * 2 + 25 && g > 20 && b as u32 > g as u32 * 5 / 4
        }
    }
}
fn ink([r, g, b]: [u8; 3]) -> bool {
    let hi = r.max(g).max(b);
    let lo = r.min(g).min(b);
    (lo > 145 && hi - lo < 70) || (hi > 145 && hi - lo > 60)
}
#[derive(Clone, Copy, Debug)]
struct Track {
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    rows: u32,
    kind: Background,
}

pub fn detect(image: &RgbImage) -> Vec<Label> {
    if image.width() > 1920 {
        let ratio = image.width() as f64 / 1920.0;
        let small = imageops::resize(
            image,
            1920,
            (image.height() as f64 / ratio).round().max(1.0) as u32,
            imageops::FilterType::Triangle,
        );
        return detect_inner(&small)
            .into_iter()
            .filter_map(|label| {
                let x = (label.rect.x as f64 * ratio).floor() as u32;
                let y = (label.rect.y as f64 * ratio).floor() as u32;
                let x1 = ((label.rect.x + label.rect.width) as f64 * ratio).ceil() as u32;
                let y1 = ((label.rect.y + label.rect.height) as f64 * ratio).ceil() as u32;
                let rect = Rect {
                    x,
                    y,
                    width: x1.min(image.width()) - x,
                    height: y1.min(image.height()) - y,
                };
                Some(Label {
                    rect,
                    name: name_row(
                        image,
                        rect,
                        (image.height() as f32 / 1080.0).clamp(0.5, 3.0),
                    )?,
                    background: label.background,
                    score: label.score,
                })
            })
            .collect();
    }
    detect_inner(image)
}
fn detect_inner(image: &RgbImage) -> Vec<Label> {
    let (w, h) = image.dimensions();
    if w < 40 || h < 30 {
        return vec![];
    }
    let scale = (h as f32 / 1080.0).clamp(0.5, 3.0);
    let px = |v: f32| (v * scale).round().max(1.0) as u32;
    let step = px(2.0);
    let gap = px(30.0);
    let tolerance = px(10.0);
    let mut tracks: Vec<Track> = Vec::new();
    let mut boxes = dark_edges(image, scale);
    for (kind, dark_limit) in [
        (Background::Blue, 0),
        (Background::Dark, 16),
        (Background::Dark, 32),
        (Background::Dark, 52),
    ] {
        tracks.clear();
        for y in (px(20.0)..h.saturating_sub(px(125.0))).step_by(step as usize) {
            let mut row = Vec::new();
            let mut start = None;
            let mut last = 0;
            for x in 0..w {
                let pixel = image.get_pixel(x, y).0;
                if if kind == Background::Dark {
                    pixel[0].max(pixel[1]).max(pixel[2]) < dark_limit
                } else {
                    background(pixel, kind)
                } {
                    if start.is_none() {
                        start = Some(x);
                    }
                    last = x;
                } else if start.is_some() && x - last > gap {
                    let x0 = start.take().unwrap();
                    if last + 1 - x0 >= px(55.0) && last + 1 - x0 <= px(700.0) {
                        row.push((x0, last + 1));
                    }
                }
            }
            if let Some(x0) = start {
                if last + 1 - x0 >= px(55.0) && last + 1 - x0 <= px(700.0) {
                    row.push((x0, last + 1));
                }
            }
            for (x0, x1) in row {
                // Ignore the known overlay text area; ground labels behind HUD
                // cannot supply independent readable evidence.
                if x0 > w * 83 / 100 && y < h / 3 {
                    continue;
                }
                if let Some(track) = tracks.iter_mut().find(|t| {
                    y - t.y1 <= px(12.0)
                        && t.x0.abs_diff(x0) <= tolerance
                        && t.x1.abs_diff(x1) <= tolerance
                }) {
                    track.y1 = y;
                    track.rows += 1;
                } else {
                    tracks.push(Track {
                        x0,
                        x1,
                        y0: y,
                        y1: y,
                        rows: 1,
                        kind,
                    });
                }
            }
            let mut index = 0;
            while index < tracks.len() {
                if y - tracks[index].y1 > px(12.0) {
                    let track = tracks.swap_remove(index);
                    if track.rows * step >= px(14.0) {
                        boxes.push(track);
                    }
                } else {
                    index += 1;
                }
            }
        }
        boxes.extend(tracks.iter().copied().filter(|t| t.rows * step >= px(14.0)));
    }
    boxes.sort_by_key(|b| (b.kind != Background::Blue, std::cmp::Reverse(b.rows)));
    boxes.truncate(512);
    let mut result: Vec<Label> = Vec::new();
    for b in boxes {
        let rect = Rect {
            x: b.x0,
            y: b.y0,
            width: b.x1 - b.x0,
            height: (b.y1 + step).min(h) - b.y0,
        };
        if rect.height > px(220.0)
            || rect.width < rect.height / 2
            || rect.x + rect.width >= w.saturating_sub(px(3.0))
        {
            continue;
        }
        let mut bg = 0;
        let mut total = 0;
        let mut color_bins = [0usize; 512];
        for y in (rect.y..rect.y + rect.height).step_by(step as usize) {
            for x in (rect.x..rect.x + rect.width).step_by(step as usize) {
                total += 1;
                let pixel = image.get_pixel(x, y).0;
                bg += usize::from(background(pixel, b.kind));
                if b.kind == Background::Blue {
                    color_bins[((pixel[0] as usize >> 5) << 6)
                        | ((pixel[1] as usize >> 5) << 3)
                        | (pixel[2] as usize >> 5)] += 1;
                }
            }
        }
        let ratio = bg as f32 / total.max(1) as f32;
        if ratio < 0.68
            || b.kind == Background::Blue
                && color_bins.into_iter().max().unwrap_or(0) as f32 / (total.max(1) as f32) < 0.65
        {
            continue;
        }
        let Some(name) = name_row(image, rect, scale) else {
            continue;
        };
        let score = ratio + if b.kind == Background::Blue { 0.2 } else { 0.0 };
        if let Some(old) = result.iter_mut().find(|old| overlap(old.rect, rect) > 0.65) {
            if score > old.score
                || (score - old.score).abs() < 0.05 && name.width() > old.name.width()
            {
                *old = Label {
                    rect,
                    name,
                    background: b.kind,
                    score,
                };
            }
        } else {
            result.push(Label {
                rect,
                name,
                background: b.kind,
                score,
            });
        }
        result.sort_by(|a, b| b.score.total_cmp(&a.score));
        result.truncate(8);
    }
    result.sort_by(|a, b| b.score.total_cmp(&a.score));
    result.truncate(8);
    result
}
// A translucent black panel attenuates the underlying picture: search paired
// horizontal brightness steps, rather than assuming its pixels are identical.
fn dark_edges(image: &RgbImage, scale: f32) -> Vec<Track> {
    let (w, h) = image.dimensions();
    let step = (2.0 * scale).round().max(1.0) as u32;
    let min_width = (60.0 * scale) as u32;
    let max_width = (650.0 * scale) as u32;
    let value = |x, y| {
        let p = image.get_pixel(x, y).0;
        p[0].max(p[1]).max(p[2]) as u32
    };
    let mut boxes = Vec::new();
    let mut comparisons = 0usize;
    for y in (step..h.saturating_sub((125.0 * scale) as u32)).step_by(step as usize) {
        let mut start = None;
        let mut last = 0;
        let mut hits = 0;
        for x in (0..w).step_by(step as usize) {
            let before = value(x, y - step);
            let after = value(x, y);
            if before >= 8 && after + 4 <= before && after * 10 <= before * 7 {
                if start.is_none() {
                    start = Some(x);
                    hits = 0;
                }
                last = x;
                hits += 1;
            } else if start.is_some() && x - last > (12.0 * scale) as u32 || x + step >= w {
                if let Some(x0) = start.take() {
                    let x1 = last + step;
                    let width = x1 - x0;
                    if width < min_width || width > max_width || hits * step * 100 < width * 55 {
                        continue;
                    }
                    if boxes.iter().any(|b: &Track| {
                        b.x0.abs_diff(x0) < step * 3 && b.y0.abs_diff(y) < step * 3
                    }) {
                        continue;
                    }
                    for bottom in (y + (20.0 * scale) as u32
                        ..(y + (200.0 * scale) as u32).min(h - step))
                        .step_by(step as usize)
                    {
                        let mut rising = 0;
                        let mut count = 0;
                        for px in (x0..x1.min(w)).step_by(step as usize) {
                            comparisons += 1;
                            if comparisons > 2_000_000 {
                                return boxes;
                            }
                            let a = value(px, bottom - step);
                            let b = value(px, bottom);
                            count += 1;
                            rising += u32::from(b >= a + 4 && b * 7 >= a * 10);
                        }
                        if rising * 100 >= count * 65 {
                            boxes.push(Track {
                                x0,
                                x1: x1.min(w),
                                y0: y,
                                y1: bottom,
                                rows: (bottom - y) / step,
                                kind: Background::Dark,
                            });
                            break;
                        }
                    }
                }
            }
        }
    }
    boxes
}
fn overlap(a: Rect, b: Rect) -> f32 {
    let w = (a.x + a.width)
        .min(b.x + b.width)
        .saturating_sub(a.x.max(b.x));
    let h = (a.y + a.height)
        .min(b.y + b.height)
        .saturating_sub(a.y.max(b.y));
    (w * h) as f32 / (a.width * a.height).min(b.width * b.height).max(1) as f32
}
fn name_row(image: &RgbImage, rect: Rect, scale: f32) -> Option<RgbImage> {
    let pad = (3.0 * scale).round().max(2.0) as u32;
    let mut rows = Vec::new();
    for y in rect.y..rect.y + rect.height {
        let n = (rect.x..rect.x + rect.width)
            .filter(|x| ink(image.get_pixel(*x, y).0))
            .count();
        // Horizontal ornament bars are not text.
        rows.push(n >= (rect.width / 70).max(3) as usize && n * 100 < rect.width as usize * 78);
    }
    let mut start = None;
    for i in 0..=rows.len() {
        if i < rows.len() && rows[i] {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(y0) = start.take() {
            let height = i - y0;
            if height < (11.0 * scale) as usize || height > (45.0 * scale) as usize {
                continue;
            }
            let y0 = rect.y + y0 as u32;
            let y1 = rect.y + i as u32;
            let columns: Vec<_> = (rect.x..rect.x + rect.width)
                .filter(|x| (y0..y1).any(|y| ink(image.get_pixel(*x, y).0)))
                .collect();
            let (&x0, &x1) = (columns.first()?, columns.last()?);
            // A partial background rectangle must not turn a clipped bilingual
            // label into a seemingly valid standalone Chinese/English name.
            if x0 < rect.x + pad || x1 + pad >= rect.x + rect.width {
                continue;
            }
            if x1 - x0 < height as u32 * 2 {
                continue;
            }
            // Reject near-solid stripes and single blobs before any OCR.
            let transitions = columns.windows(2).filter(|v| v[1] > v[0] + 1).count();
            if transitions < 2 {
                continue;
            }
            let x = x0.saturating_sub(pad).max(rect.x);
            let y = y0.saturating_sub(pad).max(rect.y);
            return Some(
                imageops::crop_imm(
                    image,
                    x,
                    y,
                    (x1 + pad + 1).min(rect.x + rect.width) - x,
                    (y1 + pad).min(rect.y + rect.height) - y,
                )
                .to_image(),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uniform_backgrounds_without_text_are_not_labels() {
        for color in [[0, 68, 131], [8, 7, 4], [100, 100, 100]] {
            assert!(detect(&RgbImage::from_pixel(640, 360, image::Rgb(color))).is_empty());
        }
    }
}
