//! Opt-in structured diagnostics in the existing application log. No image files.
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::{Duration, Instant},
};

const MAX_RECORDS_PER_MINUTE: u32 = 120;
struct Budget {
    start: Instant,
    written: u32,
    suppressed: u32,
}
impl Budget {
    fn take(&mut self, now: Instant) -> Option<u32> {
        if now.duration_since(self.start) >= Duration::from_secs(60) {
            self.start = now;
            self.written = 0;
        }
        if self.written >= MAX_RECORDS_PER_MINUTE {
            self.suppressed += 1;
            return None;
        }
        self.written += 1;
        Some(std::mem::take(&mut self.suppressed))
    }
}
pub fn event(enabled: bool, event: &str, fields: Value) {
    if !enabled {
        return;
    }
    static BUDGET: OnceLock<Mutex<Budget>> = OnceLock::new();
    let now = Instant::now();
    let skipped = BUDGET
        .get_or_init(|| {
            Mutex::new(Budget {
                start: now,
                written: 0,
                suppressed: 0,
            })
        })
        .lock()
        .take(now);
    if let Some(skipped) = skipped {
        crate::logger::log_msg(
            "INFO",
            "VisionDiagnostic",
            &json!({"event":event,"suppressed":skipped,"data":fields}).to_string(),
        );
    }
}
pub fn short(text: &str) -> String {
    text.chars().take(160).collect()
}
pub struct Trace {
    enabled: bool,
    id: u64,
    kind: &'static str,
    pid: u32,
    started: Instant,
    stages: BTreeMap<&'static str, f64>,
    pub fields: Value,
    samples: Vec<Value>,
}
impl Trace {
    pub fn new(enabled: bool, kind: &'static str, pid: u32) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            enabled,
            id: if enabled {
                NEXT.fetch_add(1, Ordering::Relaxed)
            } else {
                0
            },
            kind,
            pid,
            started: Instant::now(),
            stages: BTreeMap::new(),
            fields: json!({"outcome":"unfinished"}),
            samples: Vec::new(),
        }
    }
    pub fn measure<T>(&mut self, stage: &'static str, work: impl FnOnce() -> T) -> T {
        if !self.enabled {
            return work();
        }
        let start = Instant::now();
        let result = work();
        *self.stages.entry(stage).or_default() += start.elapsed().as_secs_f64() * 1000.0;
        result
    }
    pub fn measure_result<T>(
        &mut self,
        stage: &'static str,
        work: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let result = self.measure(stage, work);
        if let Err(error) = &result {
            self.fields["outcome"] = json!("error");
            self.fields["error"] = json!(short(error));
        }
        result
    }
    pub fn sample(&mut self, text: &str, score: f32, binary: bool) {
        if self.enabled && self.samples.len() < 8 {
            self.samples
                .push(json!({"text":short(text),"score":score,"binary":binary}));
        }
    }
}
impl Drop for Trace {
    fn drop(&mut self) {
        if self.enabled {
            event(
                true,
                "trace",
                json!({"id":self.id,"kind":self.kind,"pid":self.pid,
                "total_ms":self.started.elapsed().as_secs_f64()*1000.0,"stages_ms":self.stages,"fields":self.fields,"ocr_samples":self.samples}),
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budget_reports_suppression_after_window_rolls() {
        let now = Instant::now();
        let mut b = Budget {
            start: now,
            written: 0,
            suppressed: 0,
        };
        for _ in 0..MAX_RECORDS_PER_MINUTE {
            assert_eq!(b.take(now), Some(0));
        }
        assert_eq!(b.take(now), None);
        assert_eq!(b.take(now), None);
        assert_eq!(b.take(now + Duration::from_secs(60)), Some(2));
    }
    #[test]
    fn disabled_trace_still_runs_work_and_utf8_is_bounded() {
        let mut trace = Trace::new(false, "chat", 0);
        assert_eq!(trace.measure("capture", || 7), 7);
        assert!(trace.stages.is_empty());
        assert_eq!(short(&"测".repeat(200)).chars().count(), 160);
    }
}
