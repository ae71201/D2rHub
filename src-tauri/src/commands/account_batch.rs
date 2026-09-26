//! Read-only launch health and identity-checked, account-scoped termination.
use crate::commands::account::AccountManager;
use crate::launch_context::{ContextPurpose, LaunchContext};
use crate::state::{AccountLifecycleLease, SharedState};
use serde::Serialize;

#[derive(Serialize)]
pub struct AccountHealth {
    account_id: String,
    error: Option<String>,
}

#[tauri::command(async)]
pub fn inspect_account_launch_health(
    state: tauri::State<'_, SharedState>,
) -> Result<Vec<AccountHealth>, String> {
    let config = state.configuration().snapshot().ok_or("尚未完成首次配置")?;
    Ok(AccountManager::list_ids(&config.accounts_dir)
        .into_iter()
        .map(|id| {
            let error = AccountManager::load_meta(&config.accounts_dir, &id)
                .and_then(|meta| {
                    super::launch::preflight_account_meta(
                        &config,
                        &meta,
                        ContextPurpose::LaunchGame,
                    )
                })
                .err()
                .map(|error| error.to_string());
            AccountHealth {
                account_id: id,
                error,
            }
        })
        .collect())
}

#[derive(Serialize)]
pub struct AccountCloseResult {
    account_id: String,
    error: Option<String>,
}

#[tauri::command(async)]
pub fn close_selected_accounts(
    state: tauri::State<'_, SharedState>,
    account_ids: Vec<String>,
) -> Result<Vec<AccountCloseResult>, String> {
    let config = state.configuration().snapshot().ok_or("尚未完成首次配置")?;
    let mut seen = std::collections::HashSet::new();
    let mut results = Vec::new();
    for id in account_ids {
        if !seen.insert(id.to_ascii_lowercase()) {
            continue;
        }
        let result = (|| -> Result<(), String> {
            AccountManager::validate_account_id(&id).map_err(|e| e.to_string())?;
            let _lease = AccountLifecycleLease::try_acquire(state.inner(), &id)
                .map_err(|e| e.to_string())?;
            let meta =
                AccountManager::load_meta(&config.accounts_dir, &id).map_err(|e| e.to_string())?;
            let context = LaunchContext::for_account(&config, &meta, ContextPurpose::LaunchGame)
                .map_err(|e| e.to_string())?;
            let registry = state.multi_instance().instances();
            let instance = registry
                .get(&id)
                .ok_or("账号已退出或无法确认进程归属，请刷新后重试")?;
            let title = if meta.display_name.is_empty() {
                &id
            } else {
                &meta.display_name
            };
            crate::infrastructure::account_process::terminate_verified(
                instance.pid,
                title,
                &context.installation.game_executable,
            )?;
            registry.remove_if_pid(&id, instance.pid);
            Ok(())
        })();
        results.push(AccountCloseResult {
            account_id: id,
            error: result.err(),
        });
    }
    Ok(results)
}
