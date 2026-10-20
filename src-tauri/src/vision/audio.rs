use super::{Candidate, Kind, Vision};
use crate::rune_audio::catalog::{LocationCatalog, TelemetryMarker, MAX_AREA_ID};
use std::{
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};
use tauri::Manager;
pub enum Evidence {
    Area(Candidate),
    Drop(TelemetryMarker, f32),
}
struct Pending {
    epoch: u64,
    config_epoch: u64,
    marker: Option<TelemetryMarker>,
    confidence: f32,
    enforce: bool,
    created: Instant,
    rx: mpsc::Receiver<Result<Option<Candidate>, String>>,
}
#[derive(Default)]
pub struct AudioVision {
    epoch: u64,
    pending: Vec<Pending>,
    next_area: Option<Instant>,
    area_candidates: Option<Vec<Candidate>>,
}
impl AudioVision {
    pub fn change_scene(&mut self) {
        if !self.pending.is_empty() {
            super::diagnostics::event(
                super::configuration().diagnostic_enabled,
                "pending_invalidated",
                serde_json::json!({"reason":"scene_changed","count":self.pending.len(),"scene_epoch":self.epoch}),
            );
        }
        self.epoch += 1;
        self.pending.clear();
    }
    pub fn tick_area(&mut self, app: &tauri::AppHandle, pid: u32, catalog: &LocationCatalog) {
        if !super::configuration().area_enabled
            || self.next_area.is_some_and(|next| Instant::now() < next)
        {
            return;
        }
        self.next_area = Some(Instant::now() + Duration::from_secs(1));
        if self.pending.iter().any(|p| p.marker.is_none()) {
            return;
        }
        let candidates = self
            .area_candidates
            .get_or_insert_with(|| {
                (1..=MAX_AREA_ID)
                    .filter(|id| catalog.has_name(*id))
                    .filter_map(|area_id| catalog.resolve(TelemetryMarker::Area { area_id }))
                    .map(|v| {
                        let aliases = vec![
                            v.scene_name.clone(),
                            v.scene_name_en.clone(),
                            format!("进入{}", v.scene_name),
                            format!("Entering {}", v.scene_name_en),
                        ];
                        Candidate {
                            name: v.scene_name.clone(),
                            name_en: v.scene_name_en.clone(),
                            aliases,
                            location: Some(v),
                            tz: false,
                            confidence: 0.0,
                            announcement: None,
                        }
                    })
                    .collect()
            })
            .clone();
        self.submit(app, pid, Kind::Area(candidates), None, 0.0, false);
    }
    /// True means the audio event must wait for confirmation. Observe-only never blocks it.
    pub fn drop(
        &mut self,
        app: &tauri::AppHandle,
        pid: u32,
        marker: TelemetryMarker,
        confidence: f32,
        aliases: Vec<String>,
    ) -> bool {
        let c = super::configuration();
        if !c.drops_enabled || !c.labels_visible || confidence >= c.drop_threshold {
            return false;
        }
        self.submit(
            app,
            pid,
            Kind::Drop(aliases),
            Some(marker),
            confidence,
            c.drops_enforce,
        );
        c.drops_enforce
    }
    fn submit(
        &mut self,
        app: &tauri::AppHandle,
        pid: u32,
        kind: Kind,
        marker: Option<TelemetryMarker>,
        confidence: f32,
        enforce: bool,
    ) {
        let diagnostic = super::configuration().diagnostic_enabled;
        if self.pending.len() >= 8 {
            super::diagnostics::event(
                diagnostic,
                "submit_rejected",
                serde_json::json!({"pid":pid,"reason":"pending_full","marker":marker}),
            );
            return;
        }
        let Some(service) = app.try_state::<Arc<Vision>>() else {
            return;
        };
        match service.submit(pid, kind, self.epoch) {
            Ok(rx) => {
                super::diagnostics::event(
                    diagnostic,
                    "submitted",
                    serde_json::json!({"pid":pid,"marker":marker,"audio_score":confidence,"enforce":enforce,"scene_epoch":self.epoch}),
                );
                self.pending.push(Pending {
                    epoch: self.epoch,
                    config_epoch: super::config_epoch(),
                    marker,
                    confidence,
                    enforce,
                    created: Instant::now(),
                    rx,
                });
            }
            Err(error) => {
                super::diagnostics::event(
                    diagnostic,
                    "submit_rejected",
                    serde_json::json!({"pid":pid,"marker":marker,"error":super::diagnostics::short(&error)}),
                );
                service.status.lock().message = error;
            }
        }
    }
    pub fn poll(&mut self) -> Vec<Evidence> {
        let c = super::configuration();
        let mut out = Vec::new();
        self.pending.retain(|p| {
            if p.epoch != self.epoch
                || p.config_epoch != super::config_epoch()
                || p.created.elapsed() > Duration::from_secs(6)
            {
                super::diagnostics::event(c.diagnostic_enabled,"result_discarded",serde_json::json!({"marker":p.marker,"scene_epoch":p.epoch,"age_ms":p.created.elapsed().as_secs_f64()*1000.0,"reason":if p.epoch!=self.epoch {"scene_changed"}else if p.config_epoch!=super::config_epoch(){"config_changed"}else{"timeout"}}));
                return false;
            }
            match p.rx.try_recv() {
                Ok(Ok(Some(candidate))) => {
                    super::diagnostics::event(c.diagnostic_enabled,"result_consumed",serde_json::json!({"marker":p.marker,"name":super::diagnostics::short(&candidate.name),"action":if p.marker.is_none(){"area_name_update"}else if p.enforce && c.drops_enabled && c.drops_enforce{"confirmed_drop"}else{"observation_only"}}));
                    if let Some(marker) = p.marker {
                        if p.enforce && c.drops_enabled && c.drops_enforce {
                            out.push(Evidence::Drop(marker, p.confidence));
                        }
                    } else if c.area_enabled {
                        out.push(Evidence::Area(candidate));
                    }
                    false
                }
                Err(mpsc::TryRecvError::Empty) => true,
                other => {
                    let reason=match other {Ok(Ok(None))=>"unmatched",Ok(Err(_))=>"unavailable",Err(_)=>"worker_disconnected",_=>"discarded"};
                    super::diagnostics::event(c.diagnostic_enabled,"result_not_applied",serde_json::json!({"marker":p.marker,"reason":reason,"enforce":p.enforce}));
                    false
                },
            }
        });
        out
    }
}
