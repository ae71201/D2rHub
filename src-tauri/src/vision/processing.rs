#[cfg(test)]
use super::config::Roi;
use image::{imageops, RgbImage};

#[derive(Clone, Copy)]
pub enum Mask {
    Text,
}
#[cfg(test)]
pub fn crop(image: &RgbImage, roi: Roi) -> RgbImage {
    let x = (roi.x * image.width() as f32) as u32;
    let y = (roi.y * image.height() as f32) as u32;
    let w = ((roi.width * image.width() as f32) as u32)
        .max(1)
        .min(image.width() - x);
    let h = ((roi.height * image.height() as f32) as u32)
        .max(1)
        .min(image.height() - y);
    imageops::crop_imm(image, x, y, w, h).to_image()
}
fn selected([r, g, b]: [u8; 3], kind: Mask) -> bool {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    match kind {
        Mask::Text => {
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            (min > 150 && max - min < 65) // white/grey
                || (max > 100 && max - min > 45) // colored labels
        }
    }
}
pub fn binary(image: &RgbImage, kind: Mask) -> RgbImage {
    RgbImage::from_fn(image.width(), image.height(), |x, y| {
        image::Rgb(if selected(image.get_pixel(x, y).0, kind) {
            [0, 0, 0]
        } else {
            [255, 255, 255]
        })
    })
}
// V3: the marker is detected geometrically; only the short suffix is read by OCR.
pub fn chat_signal(sequence: u32) -> String {
    format!("+ + {sequence} + +")
}

pub fn chat_lines(image: &RgbImage) -> Vec<RgbImage> {
    lines_with_gap(image, Mask::Text, 20, 5)
}
fn lines_with_gap(image: &RgbImage, kind: Mask, limit: usize, gap: usize) -> Vec<RgbImage> {
    let original = image;
    let reduced;
    let image = if image.width() > 1920 {
        reduced = imageops::resize(
            image,
            1920,
            (image.height() as f64 * 1920.0 / image.width() as f64)
                .round()
                .max(1.0) as u32,
            imageops::FilterType::Triangle,
        );
        &reduced
    } else {
        image
    };
    let sx = original.width() as f64 / image.width() as f64;
    let sy = original.height() as f64 / image.height() as f64;
    let (w, h) = (image.width() as usize, image.height() as usize);
    let mut mask: Vec<bool> = image.pixels().map(|p| selected(p.0, kind)).collect();
    let mut boxes = Vec::<(usize, usize, usize, usize)>::new();
    let mut stack = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if !mask[y * w + x] {
                continue;
            }
            mask[y * w + x] = false;
            stack.push((x, y));
            let (mut x0, mut y0, mut x1, mut y1, mut n) = (x, y, x, y, 0);
            while let Some((px, py)) = stack.pop() {
                n += 1;
                x0 = x0.min(px);
                x1 = x1.max(px);
                y0 = y0.min(py);
                y1 = y1.max(py);
                for (nx, ny) in [
                    (px.wrapping_sub(1), py),
                    (px + 1, py),
                    (px, py.wrapping_sub(1)),
                    (px, py + 1),
                ] {
                    if nx < w && ny < h && mask[ny * w + nx] {
                        mask[ny * w + nx] = false;
                        stack.push((nx, ny));
                    }
                }
            }
            if n >= 3
                && (y1 - y0 >= 3 || (gap >= 3 && x1 - x0 >= 3))
                && y1 - y0 <= h.min(100)
                && x1 - x0 <= 200
            {
                boxes.push((x0, y0, x1 + 1, y1 + 1));
            }
            if boxes.len() > 8000 {
                return Vec::new();
            }
        }
    }
    boxes.sort_by_key(|b| b.0);
    let mut rows = Vec::<(usize, usize, usize, usize)>::new();
    for b in boxes {
        if let Some(row) = rows.iter_mut().find(|r| {
            let height = (r.3 - r.1).max(b.3 - b.1);
            let overlap = r.3.min(b.3).saturating_sub(r.1.max(b.1));
            overlap * 2 >= (r.3 - r.1).min(b.3 - b.1) && b.0 <= r.2 + height * gap
        }) {
            row.0 = row.0.min(b.0);
            row.1 = row.1.min(b.1);
            row.2 = row.2.max(b.2);
            row.3 = row.3.max(b.3);
        } else {
            rows.push(b);
        }
    }
    rows.retain(|r| r.2 - r.0 >= 18 && r.2 - r.0 >= r.3 - r.1 && r.3 - r.1 >= 6);
    rows.sort_by_key(|r| (r.1, r.0));
    let skip = rows.len().saturating_sub(limit);
    rows.into_iter()
        .skip(skip)
        .take(limit)
        .map(|r| {
            let x = r.0.saturating_sub(3);
            let y = r.1.saturating_sub(3);
            let x0 = (x as f64 * sx) as u32;
            let y0 = (y as f64 * sy) as u32;
            let x1 = (((r.2 + 3).min(w) as f64 * sx).ceil() as u32).min(original.width());
            let y1 = (((r.3 + 3).min(h) as f64 * sy).ceil() as u32).min(original.height());
            imageops::crop_imm(original, x0, y0, x1 - x0, y1 - y0).to_image()
        })
        .collect()
}
pub fn normalize(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
/// Confirm spaced pairs of crosses on both sides before OCR reads the middle.
#[derive(Debug, serde::Serialize)]
pub struct ChatAnalysis {
    pub matched: bool,
    pub glyph_runs: usize,
    pub crosses: usize,
    pub geometry_windows: usize,
    pub reason: &'static str,
}
#[cfg(test)]
pub fn chat_hint(image: &RgbImage) -> bool {
    chat_analysis(image).matched
}
pub fn chat_analysis(image: &RgbImage) -> ChatAnalysis {
    let (w, h) = (image.width(), image.height());
    let columns: Vec<bool> = (0..w)
        .map(|x| (0..h).any(|y| selected(image.get_pixel(x, y).0, Mask::Text)))
        .collect();
    let mut crosses = Vec::new();
    let mut runs = Vec::new();
    let mut x = 0;
    while x < w {
        if !columns[x as usize] {
            x += 1;
            continue;
        }
        let start = x;
        while x < w && columns[x as usize] {
            x += 1;
        }
        let width = x - start;
        runs.push((start, x));
        crosses.push(None);
        if !(3..=50).contains(&width) {
            continue;
        }
        let rows: Vec<u32> = (0..h)
            .filter(|y| (start..x).any(|px| selected(image.get_pixel(px, *y).0, Mask::Text)))
            .collect();
        let (Some(top), Some(bottom)) = (rows.first(), rows.last()) else {
            continue;
        };
        let height = bottom - top + 1;
        if width * 2 < height || height * 2 < width || height < 3 {
            continue;
        }
        // Antialiasing/shadow pixels can shift the bounding box by a pixel.
        // Search only the central half for the actual horizontal/vertical bars.
        let cy = (*top + height / 4..=*bottom - height / 4)
            .max_by_key(|y| {
                (start..x)
                    .filter(|px| selected(image.get_pixel(*px, *y).0, Mask::Text))
                    .count()
            })
            .unwrap_or((top + bottom) / 2);
        let cx = (start + width / 4..x - width / 4)
            .max_by_key(|px| {
                (*top..=*bottom)
                    .filter(|y| selected(image.get_pixel(*px, *y).0, Mask::Text))
                    .count()
            })
            .unwrap_or((start + x - 1) / 2);
        let horizontal = (start..x)
            .filter(|px| selected(image.get_pixel(*px, cy).0, Mask::Text))
            .count();
        let vertical = (*top..=*bottom)
            .filter(|py| selected(image.get_pixel(cx, *py).0, Mask::Text))
            .count();
        if horizontal * 4 >= width as usize * 3 && vertical * 4 >= height as usize * 3 {
            let ink = (*top..=*bottom)
                .flat_map(|y| (start..x).map(move |x| (x, y)))
                .filter(|(x, y)| selected(image.get_pixel(*x, *y).0, Mask::Text))
                .count();
            if ink * 100 <= (width * height) as usize * 65 {
                *crosses.last_mut().expect("glyph slot") = Some((start, x, *top, *bottom + 1));
            }
        }
    }
    let mut geometry_windows = 0;
    let mut matched = false;
    'search: for left in 0..crosses.len().saturating_sub(3) {
        let (Some(a), Some(b)) = (crosses[left], crosses[left + 1]) else {
            continue;
        };
        for right in left + 3..(left + 13).min(crosses.len().saturating_sub(1)) {
            let (Some(c), Some(d)) = (crosses[right], crosses[right + 1]) else {
                continue;
            };
            let glyphs = [a, b, c, d];
            let width = a.1 - a.0;
            let height = a.3 - a.2;
            let tolerance = (height / 4).max(2);
            if glyphs.iter().any(|g| {
                (g.1 - g.0).abs_diff(width) > tolerance
                    || (g.3 - g.2).abs_diff(height) > tolerance
                    || (g.2 + g.3).abs_diff(a.2 + a.3) > tolerance * 2
            }) {
                continue;
            }
            geometry_windows += 1;
            let first = b.0 - a.0;
            let last = d.0 - c.0;
            if first * 10 >= width * 18
                && first <= width * 5
                && last * 10 >= width * 18
                && last <= width * 5
                && first.abs_diff(last) <= (first / 4).max(2)
                && c.0 > b.1
                && c.0 - b.1 <= width * 24
                && (runs[left + 2].0 - b.1) * 2 >= width
                && (c.0 - runs[right - 1].1) * 2 >= width
            {
                matched = true;
                break 'search;
            }
        }
    }
    let cross_count = crosses.iter().filter(|g| g.is_some()).count();
    ChatAnalysis {
        matched,
        glyph_runs: crosses.len(),
        crosses: cross_count,
        geometry_windows,
        reason: if matched {
            "marker"
        } else if cross_count < 4 {
            "too_few_crosses"
        } else if geometry_windows == 0 {
            "alignment_or_shape"
        } else {
            "pair_spacing"
        },
    }
}
/// Conservative exact alias matching; do not coerce uncertain text to a rare item.
#[cfg(test)]
pub fn matches(text: &str, aliases: &[String]) -> bool {
    let text = normalize(text);
    !text.is_empty() && aliases.iter().any(|alias| text == normalize(alias))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signal_has_short_readable_dedup_suffix() {
        assert_eq!(chat_signal(501), "+ + 501 + +");
        assert_eq!(chat_signal(u32::MAX), "+ + 4294967295 + +");
    }
    fn frame(text: &str, scale: u32, stagger: bool) -> RgbImage {
        let mut image = RgbImage::new((text.len() as u32 * 10 + 8) * scale, 24 * scale);
        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            for y in 0..7 * scale {
                for x in 0..7 * scale {
                    let pixel = if ch == '+' {
                        (3 * scale..4 * scale).contains(&x) || (3 * scale..4 * scale).contains(&y)
                    } else {
                        true
                    };
                    if pixel {
                        image.put_pixel(
                            (4 + i as u32 * 10) * scale + x,
                            (4 + if stagger && i > text.len() / 2 { 7 } else { 0 }) * scale + y,
                            image::Rgb([255, 255, 255]),
                        );
                    }
                }
            }
        }
        image
    }
    #[test]
    fn requires_both_spaced_pairs_on_same_baseline() {
        for scale in [1, 2, 3] {
            let image = frame(&chat_signal(501), scale, false);
            assert!(chat_lines(&image).iter().any(chat_hint));
            for text in [
                "+501+ +",
                "+ +501+",
                "+ +501",
                "501+ +",
                "++501++",
                "+ +501+ +",
                "501",
            ] {
                assert!(!chat_hint(&frame(text, scale, false)), "{text}");
            }
            assert!(!chat_hint(&frame("+ + 501 + +", scale, true)));
        }
    }
    #[test]
    fn no_fuzzy_rare_item_promotion() {
        assert!(!matches("Ber", &["Belt".into()]));
        assert!(matches("Ber Rune", &["ber rune".into()]));
    }
}
