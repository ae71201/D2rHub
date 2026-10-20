//! A remote sequence is only a deduplication token, never a local room number.
use crate::infrastructure::module_config::ModuleConfigStore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Default, Serialize, Deserialize)]
struct History {
    scopes: BTreeMap<String, u32>,
    #[serde(default)]
    epochs: BTreeMap<String, u64>,
}
pub struct SignalHistory {
    store: ModuleConfigStore,
    pub scope: String,
    pub epoch: u64,
    pub initialized: bool,
    pub watermark: u32,
}
pub fn scope(prefix: &str, password: &str, width: u8) -> String {
    // Keep the password itself out of receipts and diagnostics; changing a rule
    // starts another stream, while changing only the local counter does not.
    let mut hash = Sha256::new();
    hash.update((prefix.len() as u64).to_le_bytes());
    hash.update(prefix.as_bytes());
    hash.update((password.len() as u64).to_le_bytes());
    hash.update(password.as_bytes());
    hash.update([width]);
    format!("{:x}", hash.finalize())
}
impl SignalHistory {
    pub fn open(root: &str, scope: String, epoch: u64) -> Result<Self, String> {
        let store =
            ModuleConfigStore::new(root, "visual-chat-history", 1).map_err(|e| e.to_string())?;
        let saved = store.load::<History>().map_err(|e| e.to_string())?;
        let watermark = saved
            .filter(|v| v.payload.epochs.get(&scope).copied().unwrap_or(0) == epoch)
            .and_then(|v| v.payload.scopes.get(&scope).copied())
            .unwrap_or(0);
        Ok(Self {
            store,
            scope,
            epoch,
            initialized: false,
            watermark,
        })
    }
    /// First valid frame establishes a baseline; later newer tokens are claimed
    /// durably before returning. Failure to persist must never dispatch input.
    pub fn observe(
        &mut self,
        tokens: impl IntoIterator<Item = u32>,
    ) -> Result<Option<u32>, String> {
        let newest = tokens.into_iter().max().unwrap_or(0);
        if newest <= self.watermark {
            self.initialized = true;
            return Ok(None);
        }
        let saved = self.store.load::<History>().map_err(|e| e.to_string())?;
        let generation = saved.as_ref().map_or(0, |v| v.generation);
        let mut history = saved.map(|v| v.payload).unwrap_or_default();
        let saved_epoch = history.epochs.get(&self.scope).copied().unwrap_or(0);
        if saved_epoch > self.epoch {
            return Err("聊天去重会话已更新，旧识别结果已丢弃".into());
        }
        let previous = if saved_epoch == self.epoch {
            history.scopes.get(&self.scope).copied().unwrap_or(0)
        } else {
            0
        };
        let trigger = self.initialized && newest > previous;
        if newest > previous {
            if !history.scopes.contains_key(&self.scope) && history.scopes.len() >= 256 {
                return Err("聊天信号去重规则记录已满，已停止自动触发".into());
            }
            history.scopes.insert(self.scope.clone(), newest);
            history.epochs.insert(self.scope.clone(), self.epoch);
            self.store
                .save_if_generation(generation, history)
                .map_err(|e| format!("保存聊天信号去重记录失败，未触发跟房：{e}"))?;
        }
        self.watermark = self.watermark.max(previous).max(newest);
        self.initialized = true;
        Ok(trigger.then_some(newest))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("vision-history-{}", uuid::Uuid::new_v4())))
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn persists_old_signal_rejection_and_does_not_require_a_local_number() {
        let dir = Directory::new();
        let root = dir.0.to_str().unwrap();
        let key = scope("zz", "1", 3);
        let mut h = SignalHistory::open(root, key.clone(), 0).unwrap();
        assert_eq!(h.observe([]).unwrap(), None);
        assert_eq!(h.observe([501]).unwrap(), Some(501));
        assert_eq!(h.observe([501]).unwrap(), None);
        assert_eq!(h.observe([499]).unwrap(), None);
        drop(h);
        let mut restarted = SignalHistory::open(root, key, 0).unwrap();
        assert_eq!(restarted.observe([]).unwrap(), None);
        assert_eq!(restarted.observe([500, 501]).unwrap(), None);
        assert_eq!(restarted.observe([501, 503, 502]).unwrap(), Some(503));
        assert_eq!(restarted.observe([502]).unwrap(), None);
    }
    #[test]
    fn visible_baseline_is_saved_and_streams_are_isolated() {
        let dir = Directory::new();
        let root = dir.0.to_str().unwrap();
        let mut h = SignalHistory::open(root, scope("a", "", 3), 0).unwrap();
        assert_eq!(h.observe([501]).unwrap(), None);
        assert_eq!(h.observe([502]).unwrap(), Some(502));
        let mut other = SignalHistory::open(root, scope("b", "", 3), 0).unwrap();
        assert_eq!(other.observe([]).unwrap(), None);
        assert_eq!(other.observe([1]).unwrap(), Some(1));
    }
    #[test]
    fn reset_replaces_old_watermark_and_preserves_baseline_across_restart() {
        let dir = Directory::new();
        let root = dir.0.to_str().unwrap();
        let key = scope("room", "pw", 3);
        let mut old = SignalHistory::open(root, key.clone(), 0).unwrap();
        assert_eq!(old.observe([900]).unwrap(), None);
        let mut reset = SignalHistory::open(root, key.clone(), 5).unwrap();
        assert_eq!(reset.watermark, 0);
        assert_eq!(reset.observe([1]).unwrap(), None);
        assert_eq!(reset.observe([2]).unwrap(), Some(2));
        assert_eq!(reset.observe([2]).unwrap(), None);
        assert!(old.observe([901]).is_err());
        let mut restarted = SignalHistory::open(root, key.clone(), 5).unwrap();
        assert_eq!(restarted.watermark, 2);
        assert_eq!(restarted.observe([]).unwrap(), None);
        assert_eq!(restarted.observe([2]).unwrap(), None);
        assert_eq!(restarted.observe([3]).unwrap(), Some(3));
        let mut again = SignalHistory::open(root, key, 6).unwrap();
        assert_eq!(again.observe([]).unwrap(), None);
        assert_eq!(again.observe([1]).unwrap(), Some(1));
        assert_eq!(
            again
                .store
                .load::<History>()
                .unwrap()
                .unwrap()
                .payload
                .scopes
                .len(),
            1
        );
    }

    #[test]
    fn corrupt_history_fails_closed() {
        let dir = Directory::new();
        let path = dir.0.join("modules/visual-chat-history");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("config.json"), b"broken").unwrap();
        assert!(SignalHistory::open(dir.0.to_str().unwrap(), scope("a", "", 3), 0).is_err());
    }
}
