//! Serializes changes to shared Mod files and processor resources.
//!
//! Generation, installation, deletion, feature edits, game launch and restart
//! all reserve the same coordinator. The lease owns its state so it can remain
//! alive across asynchronous work without exposing the underlying lock.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Default)]
pub(crate) struct ModMutationCoordinator {
    reserved: Arc<AtomicBool>,
}

pub(crate) struct ModMutationLease {
    reserved: Arc<AtomicBool>,
}

impl ModMutationCoordinator {
    pub(crate) fn try_acquire(&self) -> Result<ModMutationLease, String> {
        self.reserved
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "游戏启动、Mod 资源操作或模式切换进行中，请稍候".to_string())?;
        Ok(ModMutationLease {
            reserved: Arc::clone(&self.reserved),
        })
    }
}

impl Drop for ModMutationLease {
    fn drop(&mut self) {
        self.reserved.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;

    #[test]
    fn concurrent_mutations_have_one_owner_until_the_lease_is_released() {
        let coordinator = Arc::new(ModMutationCoordinator::default());
        let start = Arc::new(Barrier::new(8));
        let finish = Arc::new(Barrier::new(8));
        let owners = std::thread::scope(|scope| {
            let workers = (0..8)
                .map(|_| {
                    let coordinator = Arc::clone(&coordinator);
                    let start = Arc::clone(&start);
                    let finish = Arc::clone(&finish);
                    scope.spawn(move || {
                        start.wait();
                        let lease = coordinator.try_acquire().ok();
                        finish.wait();
                        lease.is_some()
                    })
                })
                .collect::<Vec<_>>();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap())
                .filter(|owned| *owned)
                .count()
        });
        assert_eq!(owners, 1);
        assert!(coordinator.try_acquire().is_ok());
    }

    #[test]
    fn failed_operation_does_not_leave_the_resource_locked() {
        let coordinator = ModMutationCoordinator::default();
        let result = std::panic::catch_unwind(|| {
            let _lease = coordinator.try_acquire().unwrap();
            panic!("simulated resource operation failure");
        });
        assert!(result.is_err());
        assert!(coordinator.try_acquire().is_ok());
    }
}
