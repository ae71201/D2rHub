//! Poll the shared token value without decrypting or logging its contents.
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use winreg::{enums::*, RegKey};

use crate::error::AppError;

pub(crate) const WEB_TOKEN_VALUE_NAME: &str = "WEB_TOKEN";

fn read_token(path: &str) -> Result<Vec<u8>, AppError> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(path, KEY_READ)
        .map_err(|error| AppError::RegistryError(format!("读取 WEB_TOKEN 路径失败: {error}")))?;
    let value = key
        .get_raw_value(WEB_TOKEN_VALUE_NAME)
        .map_err(|error| AppError::RegistryError(format!("读取 WEB_TOKEN 失败: {error}")))?;
    if value.vtype != RegType::REG_BINARY || value.bytes.is_empty() {
        return Err(AppError::RegistryError(
            "WEB_TOKEN 必须是非空二进制值".into(),
        ));
    }
    Ok(value.bytes)
}

fn observe(baseline: &[u8], sample: Result<Vec<u8>, String>) -> Result<bool, String> {
    sample.and_then(|value| {
        if value.is_empty() {
            Err("WEB_TOKEN 为空".into())
        } else {
            Ok(value != baseline)
        }
    })
}

fn count_change(
    previous: &mut Vec<u8>,
    count: &mut u8,
    sample: Result<Vec<u8>, String>,
) -> Result<bool, String> {
    let value = sample?;
    let changed = observe(previous, Ok(value.clone()))?;
    if changed {
        *previous = value;
        *count = count.saturating_add(1);
    }
    Ok(changed)
}

/// A terminal change or read failure is latched, including before PID discovery.
/// Dropping the launch scope cancels the poller on every return path.
pub(crate) struct WebTokenChangeMonitor {
    result: Arc<Mutex<Result<u8, String>>>,
    required_changes: u8,
    task: tokio::task::JoinHandle<()>,
}

impl WebTokenChangeMonitor {
    pub(crate) fn capture(
        path: &str,
        account_id: &str,
        required_changes: u8,
    ) -> Result<Self, AppError> {
        Ok(Self::start_counted(
            path.to_owned(),
            read_token(path)?,
            account_id,
            required_changes,
        ))
    }

    pub(crate) fn start(path: String, baseline: Vec<u8>, account_id: &str) -> Self {
        Self::start_counted(path, baseline, account_id, 1)
    }

    fn start_counted(
        path: String,
        baseline: Vec<u8>,
        account_id: &str,
        required_changes: u8,
    ) -> Self {
        assert!(required_changes > 0);
        let result = Arc::new(Mutex::new(Ok(0)));
        let output = result.clone();
        let account_id = account_id.to_owned();
        let started = Instant::now();
        crate::logger::log_msg(
            "INFO",
            "TokenChange",
            &format!("[Account {account_id}] WEB_TOKEN 基线已建立"),
        );
        let task = tokio::spawn(async move {
            let mut previous = baseline;
            let mut count = 0u8;
            loop {
                let sample = read_token(&path).map_err(|error| error.to_string());
                match count_change(&mut previous, &mut count, sample) {
                    Ok(did_change) => {
                        if did_change {
                            crate::logger::log_msg("INFO", "TokenChange", &format!(
                                "[Account {account_id}] WEB_TOKEN 第 {count}/{required_changes} 次变化；基线后 {}ms", started.elapsed().as_millis()));
                        }
                        *output.lock() = Ok(count);
                        if count >= required_changes {
                            break;
                        }
                    }
                    Err(error) => {
                        crate::logger::log_msg("WARN", "TokenChange", &format!(
                            "[Account {account_id}] 监测失败（已变化 {count}/{required_changes} 次）: {error}"));
                        *output.lock() = Err(error);
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(450)).await;
            }
        });
        Self {
            result,
            required_changes,
            task,
        }
    }

    pub(crate) fn change_count(&self) -> Result<u8, String> {
        let result = self.result.lock().clone();
        if matches!(result, Ok(count) if count < self.required_changes) && self.task.is_finished() {
            // Re-read after observing task completion: it may have published its
            // final count between the first read and is_finished().
            let final_result = self.result.lock().clone();
            if matches!(final_result, Ok(count) if count < self.required_changes) {
                return Err("WEB_TOKEN 监测任务提前结束，已停止启动队列".into());
            }
            return final_result;
        }
        result
    }

    pub(crate) fn changed(&self) -> Result<bool, String> {
        self.change_count()
            .map(|count| count >= self.required_changes)
    }
}

impl Drop for WebTokenChangeMonitor {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_monitor_exit_is_an_error_not_an_endless_wait() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let key = TestKey::new();
            key.write(&[1]);
            let monitor = WebTokenChangeMonitor::capture(&key.0, "test", 2).unwrap();
            monitor.task.abort();
            tokio::task::yield_now().await;
            assert!(monitor.changed().is_err());
        });
    }

    #[test]
    fn repeated_samples_do_not_count_as_a_second_change() {
        let mut previous = vec![1];
        let mut count = 0;
        assert!(count_change(&mut previous, &mut count, Ok(vec![2])).unwrap());
        for _ in 0..20 {
            assert!(!count_change(&mut previous, &mut count, Ok(vec![2])).unwrap());
        }
        assert_eq!(count, 1);
        assert!(count_change(&mut previous, &mut count, Err("read failed".into())).is_err());
        assert!(count_change(&mut previous, &mut count, Ok(vec![])).is_err());
        assert_eq!(count, 1);
        assert!(count_change(&mut previous, &mut count, Ok(vec![3])).unwrap());
        assert_eq!(count, 2);
    }

    #[test]
    fn battle_net_keeps_polling_after_first_change() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let key = TestKey::new();
            key.write(&[1]);
            let monitor = WebTokenChangeMonitor::capture(&key.0, "test-bnet", 2).unwrap();
            key.write(&[2]);
            tokio::time::timeout(Duration::from_secs(3), async {
                while monitor.change_count().unwrap() < 1 {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            })
            .await
            .unwrap();
            assert_eq!(monitor.changed(), Ok(false));
            tokio::time::sleep(Duration::from_millis(500)).await;
            assert_eq!(monitor.change_count(), Ok(1));
            key.write(&[3]);
            tokio::time::timeout(Duration::from_secs(3), async {
                while !monitor.changed().unwrap() {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            })
            .await
            .unwrap();
            assert_eq!(monitor.change_count(), Ok(2));
        });
    }

    struct TestKey(String);

    impl TestKey {
        fn new() -> Self {
            Self(format!(r"Software\D2rHubTests\{}", uuid::Uuid::new_v4()))
        }

        fn write(&self, bytes: &[u8]) {
            let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey(&self.0)
                .unwrap();
            key.set_raw_value(
                WEB_TOKEN_VALUE_NAME,
                &winreg::RegValue {
                    bytes: bytes.to_vec(),
                    vtype: RegType::REG_BINARY,
                },
            )
            .unwrap();
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.0);
        }
    }

    #[test]
    fn background_change_is_latched_before_consumer_starts_waiting() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let key = TestKey::new();
            key.write(&[1, 2]);
            let monitor = WebTokenChangeMonitor::capture(&key.0, "test", 1).unwrap();
            tokio::task::yield_now().await;
            assert_eq!(monitor.changed(), Ok(false));
            key.write(&[2, 1]);
            tokio::time::timeout(Duration::from_secs(3), async {
                while !monitor.changed().unwrap() {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            })
            .await
            .unwrap();
            key.write(&[1, 2]);
            tokio::time::sleep(Duration::from_millis(500)).await;
            assert_eq!(monitor.changed(), Ok(true));
            drop(monitor);
        });
    }

    #[test]
    fn registry_read_error_latches_and_drop_aborts_poller() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let key = TestKey::new();
            key.write(&[1]);
            let monitor = WebTokenChangeMonitor::capture(&key.0, "test", 1).unwrap();
            let abort = monitor.task.abort_handle();
            drop(monitor);
            tokio::task::yield_now().await;
            assert!(abort.is_finished());
            let monitor = WebTokenChangeMonitor::capture(&key.0, "test", 1).unwrap();
            key.write(&[]);
            tokio::task::yield_now().await;
            assert!(monitor.changed().is_err());
            key.write(&[2]);
            tokio::task::yield_now().await;
            assert!(monitor.changed().is_err());
        });
    }

    #[test]
    fn injected_value_is_not_a_login_change() {
        assert_eq!(observe(&[1, 2, 3], Ok(vec![1, 2, 3])), Ok(false));
        // Compare ordered bytes, not an unordered collection of byte values.
        assert_eq!(observe(&[1, 2, 3], Ok(vec![3, 2, 1])), Ok(true));
    }

    #[test]
    fn missing_or_invalid_value_never_confirms_login() {
        assert!(observe(&[1], Ok(vec![])).is_err());
        assert!(observe(&[1], Err("read failed".into())).is_err());
    }
}
