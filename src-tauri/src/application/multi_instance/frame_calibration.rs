use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;

/// Cheap window metadata identifies a calibration group without measuring any
/// border. Window DPI and monitor DPI differ for system-DPI-aware clients.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameCalibrationKey {
    pub monitor_id: String,
    pub monitor_dpi: u32,
    pub window_dpi: u32,
    pub style: u32,
    pub ex_style: u32,
}

/// One representative per environment in this application session. Persisted
/// metrics remain usable immediately; a new session verifies them once again.
#[derive(Default)]
pub struct FrameCalibrations {
    // false: a representative owns the work; true: successfully calibrated.
    groups: Mutex<HashMap<FrameCalibrationKey, bool>>,
}

pub enum FrameCalibrationClaim {
    Ready,
    Busy,
    Representative(FrameCalibrationLease),
}

pub struct FrameCalibrationLease {
    coordinator: Arc<FrameCalibrations>,
    key: FrameCalibrationKey,
}

impl FrameCalibrations {
    pub fn claim(self: &Arc<Self>, key: FrameCalibrationKey) -> FrameCalibrationClaim {
        let mut groups = self.groups.lock();
        match groups.get(&key) {
            Some(true) => FrameCalibrationClaim::Ready,
            Some(false) => FrameCalibrationClaim::Busy,
            None => {
                groups.insert(key.clone(), false);
                FrameCalibrationClaim::Representative(FrameCalibrationLease {
                    coordinator: self.clone(),
                    key,
                })
            }
        }
    }
}

impl FrameCalibrationLease {
    pub fn key(&self) -> &FrameCalibrationKey {
        &self.key
    }

    pub fn complete(self) {
        self.coordinator
            .groups
            .lock()
            .insert(self.key.clone(), true);
    }
}

impl Drop for FrameCalibrationLease {
    fn drop(&mut self) {
        let mut groups = self.coordinator.groups.lock();
        // Exit, cancellation, timeout or a failed measurement lets another
        // window take over. Only successful completion is reusable.
        if groups.get(&self.key) == Some(&false) {
            groups.remove(&self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> FrameCalibrationKey {
        FrameCalibrationKey {
            monitor_id: "main".into(),
            monitor_dpi: 96,
            window_dpi: 96,
            style: 1,
            ex_style: 0,
        }
    }

    #[test]
    fn concurrent_windows_elect_one_representative_and_reuse_its_result() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Barrier,
        };
        let coordinator = Arc::new(FrameCalibrations::default());
        let barrier = Barrier::new(8);
        let representatives = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    let claim = coordinator.claim(key());
                    barrier.wait();
                    match claim {
                        FrameCalibrationClaim::Representative(lease) => {
                            representatives.fetch_add(1, Ordering::SeqCst);
                            lease.complete();
                        }
                        FrameCalibrationClaim::Busy => {}
                        FrameCalibrationClaim::Ready => {
                            panic!("completion happens after all claims")
                        }
                    }
                });
            }
        });
        assert_eq!(representatives.load(Ordering::SeqCst), 1);
        assert!(matches!(
            coordinator.claim(key()),
            FrameCalibrationClaim::Ready
        ));
    }

    #[test]
    fn failed_or_abandoned_representative_releases_group_for_another_window() {
        let coordinator = Arc::new(FrameCalibrations::default());
        let FrameCalibrationClaim::Representative(lease) = coordinator.claim(key()) else {
            panic!()
        };
        assert!(matches!(
            coordinator.claim(key()),
            FrameCalibrationClaim::Busy
        ));
        drop(lease);
        let FrameCalibrationClaim::Representative(replacement) = coordinator.claim(key()) else {
            panic!()
        };
        replacement.complete();
        assert!(matches!(
            coordinator.claim(key()),
            FrameCalibrationClaim::Ready
        ));
    }

    #[test]
    fn monitor_dpi_and_window_style_groups_are_independent() {
        let coordinator = Arc::new(FrameCalibrations::default());
        let FrameCalibrationClaim::Representative(lease) = coordinator.claim(key()) else {
            panic!()
        };
        lease.complete();
        for different in [
            FrameCalibrationKey {
                monitor_id: "left".into(),
                ..key()
            },
            FrameCalibrationKey {
                monitor_dpi: 144,
                ..key()
            },
            FrameCalibrationKey {
                window_dpi: 144,
                ..key()
            },
            FrameCalibrationKey { style: 2, ..key() },
            FrameCalibrationKey {
                ex_style: 1,
                ..key()
            },
        ] {
            assert!(matches!(
                coordinator.claim(different),
                FrameCalibrationClaim::Representative(_)
            ));
        }
    }
}
