use super::{Candidate, Kind, Vision};
use crate::rune_audio::catalog::{scene_name_identity, LocationCatalog, TelemetryMarker};
use std::{
    collections::HashMap,
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

fn area_candidates(catalog: &LocationCatalog) -> Vec<Candidate> {
    let mut scenes = HashMap::new();
    for location in catalog.named_locations() {
        let mut aliases = super::area_names::aliases(&location.scene_name_en);
        aliases.extend([location.scene_name.clone(), location.scene_name_en.clone()]);
        let key = (scene_name_identity(&location.scene_name), location.kind);
        // Keep all known translations, even when distinct directory entries use
        // different English names for the same displayed scene.
        let candidate = scenes.entry(key).or_insert_with(|| Candidate {
            name: location.scene_name.clone(),
            name_en: location.scene_name_en.clone(),
            location: Some(location),
            ..Default::default()
        });
        candidate.aliases.extend(aliases);
    }
    let mut scenes: Vec<_> = scenes.into_values().collect();
    for candidate in &mut scenes {
        candidate.aliases.sort();
        candidate.aliases.dedup();
    }
    scenes.sort_by(|a, b| a.name.cmp(&b.name));
    scenes
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
            .get_or_insert_with(|| area_candidates(catalog))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rune_audio::catalog::{
        AreaCatalogEntry, AreaCatalogFile, LocationKind, AREA_CATALOG_FILE_NAME,
    };

    fn catalog_with_names(names: &[(u32, &str, &str)]) -> LocationCatalog {
        let directory = std::env::temp_dir().join(format!("named-scenes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let file = AreaCatalogFile {
            protocol_version: crate::rune_audio::protocol::PROTOCOL_VERSION,
            source_levels: "named scene regression fixture".into(),
            areas: names
                .iter()
                .map(|(id, name, english)| AreaCatalogEntry {
                    area_id: *id,
                    scene_key: format!("area_{id}"),
                    scene_name: (*name).into(),
                    scene_name_en: (*english).into(),
                    kind: if *id == 40 {
                        LocationKind::Town
                    } else {
                        LocationKind::Wilderness
                    },
                })
                .collect(),
        };
        let path = directory.join(AREA_CATALOG_FILE_NAME);
        std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();
        let catalog = LocationCatalog::load_from_directory(&directory).unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
        catalog
    }

    #[test]
    fn log_regression_tombs_and_sewers_split_timers_by_name_without_area_ids() {
        use crate::rune_audio::tracking::SegmentTracker;
        let catalog = catalog_with_names(&[
            (40, "鲁·高因", "Lut Gholein"),
            (46, "法师峡谷", "Canyon of the Magi"),
            (47, "下水道一层", "Sewers Level 1"),
            (66, "塔·拉夏之墓", "Tal Rasha's Tomb"),
            (67, "塔·拉夏之墓", "Tal Rasha's Tomb"),
            (68, "塔·拉夏之墓", "Tal Rasha's Tomb"),
            (69, "塔·拉夏之墓", "Tal Rasha's Tomb"),
            (70, "塔·拉夏之墓", "Tal Rasha's Tomb"),
            (71, "塔·拉夏之墓", "Tal Rasha's Tomb"),
            (72, "塔·拉夏之墓", "Tal Rasha's Tomb"),
            (73, "塔·拉夏的墓室", "Tal Rasha's Chamber"),
            (92, "下水道一层", "Kurast Sewers Level 1"),
        ]);
        let candidates = area_candidates(&catalog);
        for name in ["塔·拉夏之墓", "下水道一层"] {
            assert_eq!(candidates.iter().filter(|v| v.name == name).count(), 1);
        }
        let resolve = |text| {
            super::super::area::resolve(text, &candidates)
                .unwrap()
                .location
                .unwrap()
        };
        let tomb = resolve("进入塔·拉夏之墓");
        assert_eq!(resolve("进入塔拉夏之墓"), tomb);
        assert_eq!(resolve("進入塔拉夏的古墓"), tomb);
        assert_eq!(resolve("Entering Tal Rasha's Tomb"), tomb);
        assert_ne!(resolve("进入塔·拉夏的墓室"), tomb);
        let sewer = resolve("进入下水道一层");
        assert_eq!(resolve("Entering Kurast Sewers Level 1"), sewer);
        assert_eq!(resolve("Entering Sewers Level 1"), sewer);

        let mut tracker =
            SegmentTracker::with_catalog("test".into(), "test".into(), 48_000, catalog);
        let canyon = tracker.observe_visual_location(
            resolve("进入法师峡谷"),
            true,
            Some(1),
            1_000,
            "start".into(),
        );
        assert_eq!(canyon.snapshot.current_area_id, None);
        let entered =
            tracker.observe_visual_location(tomb.clone(), true, Some(2), 87_000, "tomb".into());
        let canyon = entered.completed_segment.unwrap();
        assert_eq!(canyon.scene_name, "法师峡谷");
        assert_eq!(canyon.timer_seconds, 86.0);
        assert_eq!(entered.snapshot.current_scene, "塔·拉夏之墓");
        assert_eq!(entered.snapshot.current_area_id, None);
        let repeat = tracker.observe_visual_location(tomb, true, Some(2), 88_000, "repeat".into());
        assert!(!repeat.changed);
        assert_eq!(repeat.snapshot.timer_started_at_ms, Some(87_000));
        let town = tracker.observe_visual_location(
            resolve("进入鲁高因"),
            false,
            Some(3),
            513_700,
            "town".into(),
        );
        let tomb = town.completed_segment.unwrap();
        assert_eq!(tomb.scene_name, "塔·拉夏之墓");
        assert_eq!(tomb.timer_seconds, 426.7);
        assert!(!town.snapshot.is_timing);
        assert_eq!(town.snapshot.current_area_id, None);
        tracker.observe_visual_location(sewer, false, Some(4), 600_000, "sewer".into());
        let stopped = tracker.observe_visual_location(
            resolve("进入鲁高因"),
            false,
            Some(5),
            610_000,
            "town".into(),
        );
        let sewer = stopped.completed_segment.unwrap();
        assert_eq!(sewer.scene_name, "下水道一层");
        assert_eq!(sewer.timer_seconds, 10.0);
    }

    #[test]
    fn localized_ocr_resolves_named_scenes_and_merges_duplicate_directory_entries() {
        let directory = std::env::temp_dir().join(format!("area-aliases-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let file = AreaCatalogFile {
            protocol_version: crate::rune_audio::protocol::PROTOCOL_VERSION,
            source_levels: "translation regression fixture".into(),
            areas: [
                (8, "邪恶洞穴", "Den of Evil"),
                (9, "洞穴一层", "Cave Level 1"),
                (12, "深坑一层", "Pit Level 1"),
                (16, "深坑二层", "Pit Level 2"),
                (38, "崔斯特姆", "Tristram"),
                (136, "崔斯特姆", "Tristram"),
            ]
            .into_iter()
            .map(|(area_id, chinese, english)| AreaCatalogEntry {
                area_id,
                scene_key: format!("area_{area_id}"),
                scene_name: chinese.into(),
                scene_name_en: english.into(),
                kind: LocationKind::Wilderness,
            })
            .collect(),
        };
        let path = directory.join(AREA_CATALOG_FILE_NAME);
        std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();
        let catalog = LocationCatalog::load_from_directory(&directory).unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
        let candidates = area_candidates(&catalog);
        for (text, area_id) in [
            ("进入流亡者营地", 1),
            ("进入罗格营地", 1),
            ("进入萝格营地", 1),
            ("進入俠盜營地", 1),
            ("进入深渊一层", 12),
            ("进入深坑一层", 12),
            ("进入“地穴 第一层”", 12),
            ("進入地穴第一層", 12),
            ("进入地穴第一层[07/39/85]", 12),
            ("进入深渊二层", 16),
            ("進入地穴第二層", 16),
        ] {
            let found = super::super::area::resolve(text, &candidates)
                .unwrap_or_else(|| panic!("unmatched: {text}"));
            let expected = catalog.resolve(TelemetryMarker::Area { area_id }).unwrap();
            assert_eq!(found.location.unwrap().scene_name, expected.scene_name);
        }
        let tristram = super::super::area::resolve("进入崔斯特姆", &candidates).unwrap();
        assert_eq!(tristram.location.unwrap().scene_name, "崔斯特姆");
        assert_eq!(
            candidates.iter().filter(|v| v.name == "崔斯特姆").count(),
            1
        );
        for text in [
            "深渊一层",
            "进入深渊三层",
            "进入深渊立层",
            "进入外廊",
            "进入洞穴一层",
        ] {
            assert!(
                super::super::area::resolve(text, &candidates).is_none(),
                "{text}"
            );
        }
    }
}
