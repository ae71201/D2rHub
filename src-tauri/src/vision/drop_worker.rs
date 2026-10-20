use super::{diagnostics::Trace, drop_match, labels, ocr::Ocr, Candidate, Kind, Request};
use image::RgbImage;
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Drops {
    context: Option<(u32, u64)>,
    cache: VecDeque<([u8; 32], (String, f32))>,
}
impl Drops {
    pub fn evaluate(
        &mut self,
        requests: &[Request],
        ocr: &mut Ocr,
        stop: &AtomicBool,
        trace: &mut Trace,
    ) -> Result<Vec<Option<Candidate>>, String> {
        self.read(
            requests,
            stop,
            trace,
            || super::capture::capture(requests[0].pid, None),
            |image| ocr.recognize(image),
        )
    }
    fn read(
        &mut self,
        requests: &[Request],
        stop: &AtomicBool,
        trace: &mut Trace,
        mut capture: impl FnMut() -> Result<super::capture::Captured, String>,
        mut recognize: impl FnMut(&RgbImage) -> Result<(String, f32), String>,
    ) -> Result<Vec<Option<Candidate>>, String> {
        let first = &requests[0];
        let context = (first.pid, first.context);
        if self.context != Some(context) {
            self.cache.clear();
            self.context = Some(context);
        }
        let mut results = vec![None; requests.len()];
        let deadline = Instant::now() + Duration::from_millis(1500);
        let mut calls = 0;
        trace.fields["ocr_calls"] = serde_json::json!(0);
        trace.fields["cache_hits"] = serde_json::json!(0);
        trace.fields["strategy"] = serde_json::json!("background_rectangles_rgb");
        let mut readable = false;
        let mut budget = false;
        for attempt in 0..2 {
            if stop.load(Ordering::Acquire) {
                return Err("视觉核验已停止".into());
            }
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(250));
            }
            if Instant::now() >= deadline {
                budget = true;
                break;
            }
            trace.fields["attempts"] = serde_json::json!(attempt + 1);
            let captured = match trace.measure_result("capture", &mut capture) {
                Ok(value) => value,
                Err(error) => {
                    if results.iter().any(Option::is_some) {
                        trace.fields["unconfirmed_reason"] = serde_json::json!(error);
                        return Ok(results);
                    }
                    return Err(error);
                }
            };
            trace.fields["capture_metrics"] = serde_json::json!(captured.metrics);
            let candidates = trace.measure("label_detection", || labels::detect(&captured.image));
            trace.fields["labels"] = serde_json::json!(candidates
                .iter()
                .map(
                    |v| serde_json::json!({"rect":v.rect,"background":v.background,"score":v.score})
                )
                .collect::<Vec<_>>());
            for label in candidates {
                if stop.load(Ordering::Acquire) {
                    return Err("视觉核验已停止".into());
                }
                if Instant::now() >= deadline {
                    budget = true;
                    break;
                }
                let signature = fingerprint(&label.name);
                let (text, score) = if let Some((_, value)) =
                    self.cache.iter().find(|(key, _)| *key == signature)
                {
                    trace.fields["cache_hits"] =
                        serde_json::json!(trace.fields["cache_hits"].as_u64().unwrap_or(0) + 1);
                    value.clone()
                } else {
                    if calls >= 12 {
                        budget = true;
                        break;
                    }
                    calls += 1;
                    trace.fields["ocr_calls"] = serde_json::json!(calls);
                    let value = match trace.measure_result("ocr", || recognize(&label.name)) {
                        Ok(value) => value,
                        Err(error) => {
                            trace.fields["unconfirmed_reason"] = serde_json::json!(error);
                            continue;
                        }
                    };
                    trace.sample(&value.0, value.1, false);
                    if value.1 >= 0.90 {
                        if self.cache.len() >= 64 {
                            self.cache.pop_front();
                        }
                        self.cache.push_back((signature, value.clone()));
                    }
                    value
                };
                if score < 0.90 {
                    continue;
                }
                readable = true;
                for (index, request) in requests.iter().enumerate() {
                    if let Kind::Drop(aliases) = &request.kind {
                        if results[index].is_none() && drop_match::matches(&text, aliases) {
                            results[index] = Some(Candidate {
                                name: text.clone(),
                                name_en: String::new(),
                                aliases: vec![],
                                ..Default::default()
                            });
                        }
                    }
                }
                if results.iter().all(Option::is_some) {
                    return Ok(results);
                }
            }
        }
        trace.fields["budget_exhausted"] = serde_json::json!(budget);
        // Preserve already confirmed events even when another item is unreadable.
        if results.iter().any(Option::is_some) || readable && !budget {
            return Ok(results);
        }
        Err(if budget {
            "掉落视觉核验达到处理上限，未确认项不确定"
        } else {
            "未找到可读的掉落标签，核验不确定"
        }
        .into())
    }
}
fn fingerprint(image: &RgbImage) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(image.width().to_le_bytes());
    hash.update(image.height().to_le_bytes());
    hash.update(image.as_raw());
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(number: u32, context: u64) -> Request {
        Request {
            pid: 42,
            context,
            created: Instant::now(),
            kind: Kind::Drop(crate::rune_data::visual_rune_aliases(number)),
            reply: std::sync::mpsc::sync_channel(1).0,
        }
    }
    fn fixture() -> RgbImage {
        let mut image = RgbImage::from_pixel(640, 360, image::Rgb([110, 110, 110]));
        for y in 100..160 {
            for x in 200..360 {
                image.put_pixel(x, y, image::Rgb([0, 68, 131]));
            }
        }
        for glyph in 0..7 {
            for y in 114..126 {
                for x in 0..5 {
                    if x == 0 || y == 114 || y == 125 {
                        image.put_pixel(220 + glyph * 12 + x, y, image::Rgb([255, 180, 0]));
                    }
                }
            }
        }
        image
    }
    #[test]
    fn shares_capture_and_ocr_between_events_and_invalidates_cache_on_scene_change() {
        let image = fixture();
        assert_eq!(labels::detect(&image).len(), 1);
        let mut worker = Drops::default();
        let mut captures = 0;
        let mut calls = 0;
        let mut run = |worker: &mut Drops, context| {
            worker
                .read(
                    &[request(23, context), request(23, context)],
                    &AtomicBool::new(false),
                    &mut Trace::new(false, "drop", 42),
                    || {
                        captures += 1;
                        Ok(super::super::capture::Captured {
                            image: image.clone(),
                            metrics: Default::default(),
                        })
                    },
                    |_| {
                        calls += 1;
                        Ok(("*玛尔 Mal#23".into(), 0.99))
                    },
                )
                .unwrap()
        };
        assert!(run(&mut worker, 0).iter().all(Option::is_some));
        assert!(run(&mut worker, 0).iter().all(Option::is_some));
        assert!(run(&mut worker, 1).iter().all(Option::is_some));
        assert_eq!(captures, 3);
        assert_eq!(calls, 2);
    }
    #[test]
    fn partial_confirmation_survives_a_failed_second_capture() {
        let image = fixture();
        let mut captures = 0;
        let mut trace = Trace::new(false, "drop", 42);
        let result = Drops::default()
            .read(
                &[request(23, 0), request(21, 0)],
                &AtomicBool::new(false),
                &mut trace,
                || {
                    captures += 1;
                    if captures > 1 {
                        Err("capture unavailable".into())
                    } else {
                        Ok(super::super::capture::Captured {
                            image: image.clone(),
                            metrics: Default::default(),
                        })
                    }
                },
                |_| Ok(("玛尔Mal#23".into(), 0.99)),
            )
            .unwrap();
        assert!(result[0].is_some());
        assert!(result[1].is_none());
        assert_eq!(trace.fields["unconfirmed_reason"], "capture unavailable");
    }
}
