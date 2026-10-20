//! Marker-gated OCR. The suffix is a deduplication ID, never a local room rule.
use super::{
    diagnostics::Trace,
    processing::{self, ChatAnalysis},
};
use image::RgbImage;
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
pub const SUCCESS_COOLDOWN: Duration = Duration::from_secs(5);
const FAILURE_COOLDOWN: Duration = Duration::from_secs(2);
const MAX_CALLS: usize = 4;
#[derive(Default)]
pub struct ChatOcr {
    until: Option<Instant>,
    cache: VecDeque<([u8; 32], u32)>,
}
/// The layout gate already isolated the center; never guess O/0 or I/1.
pub fn parse_number(text: &str) -> Option<u32> {
    let normalized: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '０'..='９' => char::from_u32(c as u32 - '０' as u32 + '0' as u32).unwrap(),
            _ => c,
        })
        .collect();
    let digits = normalized.as_str();
    if digits.is_empty() || digits.len() > 10 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok().filter(|value| *value > 0)
}
fn fingerprint(line: &RgbImage) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(line.width().to_le_bytes());
    hash.update(line.height().to_le_bytes());
    hash.update(processing::binary(line, processing::Mask::Text).as_raw());
    hash.finalize().into()
}

impl ChatOcr {
    #[cfg(test)]
    pub fn scan(
        &mut self,
        lines: &[RgbImage],
        reports: &[ChatAnalysis],
        now: Instant,
        recognize: impl FnMut(&RgbImage) -> Result<(String, f32), String>,
        trace: &mut Trace,
    ) -> Result<Vec<u32>, String> {
        self.scan_new(lines, reports, now, 0, recognize, trace)
    }
    pub fn scan_new(
        &mut self,
        lines: &[RgbImage],
        reports: &[ChatAnalysis],
        now: Instant,
        watermark: u32,
        mut recognize: impl FnMut(&RgbImage) -> Result<(String, f32), String>,
        trace: &mut Trace,
    ) -> Result<Vec<u32>, String> {
        trace.fields["line_analysis"] = serde_json::json!([]);
        trace.fields["present"] = serde_json::json!(false);
        trace.fields["ocr_calls"] = serde_json::json!(0);
        if let Some(until) = self.until.filter(|until| *until > now) {
            trace.fields["ocr_skip"] = serde_json::json!("cooldown");
            trace.fields["cooldown_remaining_ms"] =
                serde_json::json!(until.duration_since(now).as_millis());
            return Ok(Vec::new());
        }
        let mut tokens = Vec::new();
        let mut calls = 0;
        for (index, line) in lines.iter().enumerate().rev() {
            // Runtime analysis is lazy: never examine older rows after a new signal.
            let detected;
            let report = if let Some(report) = reports.get(index) {
                report
            } else {
                detected = trace.measure("signal_detection", || processing::chat_analysis(line));
                &detected
            };
            trace.fields["line_analysis"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({"index":index,"analysis":report}));
            let Some((x0, x1)) = report.number_bounds.filter(|_| report.matched) else {
                continue;
            };
            trace.fields["present"] = serde_json::json!(true);
            let signature = fingerprint(line);
            if let Some((_, token)) = self.cache.iter().find(|(key, _)| *key == signature) {
                tokens.push(*token);
                trace.fields["ocr_skip"] = serde_json::json!("unchanged_mask");
                if *token > watermark {
                    break;
                }
                continue;
            }
            if calls >= MAX_CALLS {
                break;
            }
            self.until = Some(now + FAILURE_COOLDOWN);
            calls += 1;
            trace.fields["ocr_calls"] = serde_json::json!(calls);
            let center = image::imageops::crop_imm(line, x0, 0, x1 - x0, line.height()).to_image();
            let (text, score) = trace.measure_result("ocr", || recognize(&center))?;
            trace.sample(&text, score, false);
            let token = (score >= 0.90).then(|| parse_number(&text)).flatten();
            if let Some(token) = token {
                if self.cache.len() >= 32 {
                    self.cache.pop_front();
                }
                self.cache.push_back((signature, token));
                tokens.push(token);
                self.until = Some(Instant::now().max(now) + SUCCESS_COOLDOWN);
                trace.fields["cooldown_ms"] = serde_json::json!(SUCCESS_COOLDOWN.as_millis());
                if token > watermark {
                    break;
                }
            }
        }
        if calls == 0 && tokens.is_empty() {
            trace.fields["ocr_skip"] = serde_json::json!("no_marker");
        }
        trace.fields["decoded_tokens"] = serde_json::json!(tokens);
        Ok(tokens)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Uses optional native OCR assets; set D2RHUB_CHAT_OCR_SAMPLE to a local compact signal PNG"]
    fn native_compact_signal_smoke() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../dev/vision-resources/cache");
        let stop = std::sync::atomic::AtomicBool::new(false);
        let files = tauri::async_runtime::block_on(super::super::resources::ensure(
            &root,
            &stop,
            |status| println!("{status}"),
        ))
        .unwrap();
        let mut ocr = super::super::ocr::Ocr::new(&files).unwrap();
        let image = image::open(std::env::var("D2RHUB_CHAT_OCR_SAMPLE").unwrap())
            .unwrap()
            .to_rgb8();
        let lines = processing::chat_lines(&image);
        let reports: Vec<_> = lines.iter().map(processing::chat_analysis).collect();
        assert!(reports.iter().any(|report| report.matched), "{reports:?}");
        let tokens = ChatOcr::default()
            .scan_new(
                &lines,
                &reports,
                Instant::now(),
                u32::MAX,
                |line| {
                    let result = ocr.recognize(line)?;
                    println!("OCR: {result:?}");
                    Ok(result)
                },
                &mut Trace::new(false, "chat", 0),
            )
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                std::env::var("D2RHUB_CHAT_OCR_EXPECT")
                    .unwrap_or_else(|_| "14".into())
                    .parse::<u32>()
                    .unwrap();
                lines.len()
            ]
        );
    }
    fn marker(width: u32) -> ChatAnalysis {
        ChatAnalysis {
            matched: true,
            glyph_runs: 7,
            marker_runs: 22,
            number_bounds: Some((0, width)),
            geometry_windows: 1,
            reason: "marker",
        }
    }
    #[test]
    fn parses_only_the_isolated_numeric_center() {
        assert_eq!(parse_number(" 077 "), Some(77));
        assert_eq!(parse_number("０１４"), Some(14));
        assert_eq!(parse_number("4294967295"), Some(u32::MAX));
        for text in [
            "",
            "5O1",
            "0",
            "4294967296",
            "501 extra",
            "ID:501",
            "***501***",
            "+501",
        ] {
            assert_eq!(parse_number(text), None, "{text}");
        }
    }
    #[test]
    fn only_markers_invoke_ocr_and_success_cools_down_for_five_seconds() {
        let now = Instant::now();
        let mut state = ChatOcr::default();
        let mut trace = Trace::new(false, "chat", 0);
        let line = RgbImage::new(100, 20);
        assert!(state
            .scan(
                std::slice::from_ref(&line),
                &[],
                now,
                |_| panic!("no marker"),
                &mut trace
            )
            .unwrap()
            .is_empty());
        assert_eq!(
            state
                .scan(
                    std::slice::from_ref(&line),
                    &[marker(100)],
                    now,
                    |_| Ok(("501".into(), 0.99)),
                    &mut trace
                )
                .unwrap(),
            vec![501]
        );
        assert!(state
            .scan(
                std::slice::from_ref(&line),
                &[marker(100)],
                now + Duration::from_secs(4),
                |_| panic!("cooldown"),
                &mut trace
            )
            .unwrap()
            .is_empty());
        assert_eq!(
            state
                .scan(
                    std::slice::from_ref(&line),
                    &[marker(100)],
                    now + Duration::from_secs(6),
                    |_| panic!("unchanged"),
                    &mut trace
                )
                .unwrap(),
            vec![501]
        );
        let changed = RgbImage::from_pixel(100, 20, image::Rgb([255, 255, 255]));
        assert_eq!(
            state
                .scan(
                    &[changed],
                    &[marker(100)],
                    now + Duration::from_secs(6),
                    |_| Ok(("502".into(), 0.99)),
                    &mut trace
                )
                .unwrap(),
            vec![502]
        );
    }
    #[test]
    fn low_scores_and_ambiguous_digits_cannot_create_tokens() {
        let now = Instant::now();
        let mut state = ChatOcr::default();
        let mut trace = Trace::new(false, "chat", 0);
        let line = RgbImage::new(100, 20);
        assert!(state
            .scan(
                std::slice::from_ref(&line),
                &[marker(100)],
                now,
                |_| Ok(("501".into(), 0.89)),
                &mut trace
            )
            .unwrap()
            .is_empty());
        assert!(state
            .scan(
                std::slice::from_ref(&line),
                &[marker(100)],
                now + Duration::from_secs(1),
                |_| panic!("failure cooldown"),
                &mut trace
            )
            .unwrap()
            .is_empty());
        assert!(state
            .scan(
                &[line],
                &[marker(100)],
                now + Duration::from_secs(3),
                |_| Ok(("5O1".into(), 0.99)),
                &mut trace
            )
            .unwrap()
            .is_empty());
    }
    #[test]
    fn scans_bottom_up_past_old_tokens_and_stops_at_first_new_token() {
        let now = Instant::now();
        let mut state = ChatOcr::default();
        let mut trace = Trace::new(false, "chat", 0);
        let lines: Vec<_> = [100, 110, 120]
            .into_iter()
            .map(|width| RgbImage::new(width, 20))
            .collect();
        let mut visited = Vec::new();
        let tokens = state
            .scan_new(
                &lines,
                &[marker(100), marker(110), marker(120)],
                now,
                501,
                |line| {
                    visited.push(line.width());
                    match line.width() {
                        120 => Ok(("501".into(), 0.99)),
                        110 => Ok(("502".into(), 0.99)),
                        _ => panic!("older rows must not be read after a new token"),
                    }
                },
                &mut trace,
            )
            .unwrap();
        assert_eq!(visited, vec![120, 110]);
        assert_eq!(tokens, vec![501, 502]);
        assert_eq!(trace.fields["line_analysis"].as_array().unwrap().len(), 2);

        // The same early stop must apply to cached results without another OCR call.
        let cached = state
            .scan_new(
                &lines,
                &[marker(100), marker(110), marker(120)],
                now + Duration::from_secs(6),
                501,
                |_| panic!("cached rows must not invoke OCR"),
                &mut trace,
            )
            .unwrap();
        assert_eq!(cached, vec![501, 502]);
        assert_eq!(trace.fields["line_analysis"].as_array().unwrap().len(), 2);
    }
    #[test]
    fn real_marker_layout_sends_only_the_numeric_crop_to_ocr() {
        for png in [
            include_bytes!("../../tests/fixtures/chat-signal-1080p.png").as_slice(),
            include_bytes!("../../tests/fixtures/chat-signal-720p.png").as_slice(),
        ] {
            let image = image::load_from_memory(png).unwrap().to_rgb8();
            let lines = processing::chat_lines(&image);
            let mut state = ChatOcr::default();
            let mut calls = 0;
            let mut trace = Trace::new(false, "chat", 0);
            let now = Instant::now();
            let tokens = state
                .scan_new(
                    &lines,
                    &[],
                    now,
                    u32::MAX,
                    |center| {
                        calls += 1;
                        assert!(center.width() < lines[0].width() / 3);
                        Ok(("014".into(), 0.99))
                    },
                    &mut trace,
                )
                .unwrap();
            assert_eq!(tokens, vec![14; 3]);
            // Some screenshot rows have identical masks and are already cached
            // within this scan, even when their backgrounds differ.
            assert!((1..=3).contains(&calls));
            assert_eq!(
                state
                    .scan_new(
                        &lines,
                        &[],
                        now + Duration::from_secs(6),
                        0,
                        |_| panic!("identical rows must use the cache"),
                        &mut trace
                    )
                    .unwrap(),
                vec![14]
            );
        }
    }
    #[test]
    fn baseline_scans_all_candidates_without_short_circuiting() {
        let lines = vec![RgbImage::new(100, 20), RgbImage::new(110, 20)];
        let mut trace = Trace::new(false, "chat", 0);
        let tokens = ChatOcr::default()
            .scan_new(
                &lines,
                &[marker(100), marker(110)],
                Instant::now(),
                u32::MAX,
                |line| Ok((format!("{}", line.width()), 0.99)),
                &mut trace,
            )
            .unwrap();
        assert_eq!(tokens, vec![110, 100]);
    }
}
