//! Sidecar persistence and recoverable account/scheme reference transactions.
use super::*;

pub(super) fn catalog_store(state: &SharedState) -> Result<ModuleConfigStore, String> {
    ModuleConfigStore::new(&state.app_data_dir, MODULE_ID, SCHEMA_VERSION)
        .map_err(|error| error.to_string())
}

pub(super) fn load_payload(
    state: &SharedState,
    config: &GlobalConfig,
    scanned: &[ScannedMod],
) -> Result<(u64, ModCatalogPayload), String> {
    let store = catalog_store(state)?;
    let loaded = store
        .load::<ModCatalogPayload>()
        .map_err(|error| error.to_string())?;
    let (generation, mut payload, missing) = match loaded {
        Some(envelope) => (envelope.generation, envelope.payload, false),
        None => (0, ModCatalogPayload::default(), true),
    };
    let migrated = if payload.legacy_import_completed {
        false
    } else {
        merge_legacy_entries(config, scanned, &mut payload);
        payload.legacy_import_completed = true;
        true
    };
    if missing || migrated {
        let saved = store
            .save_if_generation(generation, payload)
            .map_err(|error| error.to_string())?;
        Ok((saved.generation, saved.payload))
    } else {
        Ok((generation, payload))
    }
}

pub(super) fn save_payload(
    state: &SharedState,
    generation: u64,
    payload: ModCatalogPayload,
) -> Result<(u64, ModCatalogPayload), String> {
    let saved = catalog_store(state)?
        .save_if_generation(generation, payload)
        .map_err(|error| error.to_string())?;
    Ok((saved.generation, saved.payload))
}

pub(super) fn load_payload_with_recovery(
    state: &SharedState,
    app: &tauri::AppHandle,
    config: &GlobalConfig,
    scanned: &[ScannedMod],
) -> Result<(u64, ModCatalogPayload), String> {
    let (generation, payload) = load_payload(state, config, scanned)?;
    recover_argument_update(state, app, config, generation, payload)
}

pub(super) fn recover_argument_update(
    state: &SharedState,
    app: &tauri::AppHandle,
    config: &GlobalConfig,
    generation: u64,
    mut payload: ModCatalogPayload,
) -> Result<(u64, ModCatalogPayload), String> {
    let Some(pending) = payload.pending_argument_update.clone() else {
        return Ok((generation, payload));
    };

    let _account_catalog_lease = state
        .multi_instance()
        .catalog_leases()
        .acquire()
        .map_err(|error| error.to_string())?;
    let _account_leases = state
        .multi_instance()
        .account_leases()
        .try_acquire_many(
            pending
                .accounts
                .iter()
                .map(|change| change.account_id.as_str()),
        )
        .map_err(|error| format!("恢复未完成的 Mod 目录事务失败: {error}"))?;
    apply_account_mod_journal(config, &pending.accounts, false)
        .map_err(|error| format!("恢复未完成的 Mod 账号引用失败: {error}"))?;
    apply_scheme_mod_replacements(state, app, &pending.scheme_members, false)
        .map_err(|error| format!("恢复未完成的 Mod 启动方案引用失败: {error}"))?;

    payload.pending_argument_update = None;
    let saved = save_payload(state, generation, payload)?;
    crate::logger::log_msg(
        "WARN",
        "ModCatalog",
        &format!("已回滚上次中断的 Mod 目录编辑事务: {}", pending.capsule_id),
    );
    Ok(saved)
}

pub(super) fn restore_account_mod_replacements(
    config: &GlobalConfig,
    changes: &[AccountModReplacement],
) -> Vec<String> {
    let mut errors = Vec::new();
    for change in changes.iter().rev() {
        if let Err(error) = update_account_mods_with_lease_held(
            config,
            change.original.clone(),
            change.original.mod_args.clone(),
            change.original.mod_list.clone(),
        ) {
            errors.push(format!("账号 {}: {error}", change.original.id));
        }
    }
    errors
}

pub(super) fn apply_account_mod_replacements(
    config: &GlobalConfig,
    changes: &[AccountModReplacement],
) -> Result<(), String> {
    for (index, change) in changes.iter().enumerate() {
        if let Err(error) = update_account_mods_with_lease_held(
            config,
            change.original.clone(),
            change.active.clone(),
            change.list.clone(),
        ) {
            let rollback_errors = restore_account_mod_replacements(config, &changes[..index]);
            return Err(if rollback_errors.is_empty() {
                error.to_string()
            } else {
                format!(
                    "{error}；回滚已更新账号时发生错误：{}",
                    rollback_errors.join("；")
                )
            });
        }
    }
    Ok(())
}

pub(super) fn apply_account_mod_journal(
    config: &GlobalConfig,
    changes: &[AccountModJournalEntry],
    forward: bool,
) -> Result<(), String> {
    for change in changes {
        let account = AccountManager::load_meta(&config.accounts_dir, &change.account_id)
            .map_err(|error| error.to_string())?;
        let (expected_active, expected_list, target_active, target_list) = if forward {
            (
                &change.old_active,
                &change.old_list,
                &change.new_active,
                &change.new_list,
            )
        } else {
            (
                &change.new_active,
                &change.new_list,
                &change.old_active,
                &change.old_list,
            )
        };
        if &account.mod_args == target_active && &account.mod_list == target_list {
            continue;
        }
        if &account.mod_args != expected_active || &account.mod_list != expected_list {
            return Err(format!(
                "账号 {} 的 Mod 配置已被其他操作修改，停止恢复目录事务",
                change.account_id
            ));
        }
        update_account_mods_with_lease_held(
            config,
            account,
            target_active.clone(),
            target_list.clone(),
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(super) fn apply_scheme_mod_replacements(
    state: &SharedState,
    app: &tauri::AppHandle,
    changes: &[SchemeModJournalEntry],
    forward: bool,
) -> Result<(), String> {
    mutate_loaded_global_config(state, app, |config| {
        let mut changed = false;
        for change in changes {
            let member = config
                .launch_groups
                .iter_mut()
                .find(|group| group.id == change.group_id)
                .and_then(|group| {
                    group
                        .members
                        .iter_mut()
                        .find(|member| member.account_id == change.account_id)
                })
                .ok_or_else(|| {
                    crate::error::AppError::ConfigWriteError(format!(
                        "启动方案 {} 中已找不到账号 {}，停止 Mod 引用事务",
                        change.group_id, change.account_id
                    ))
                })?;
            let (expected, target) = if forward {
                (&change.old_arguments, &change.new_arguments)
            } else {
                (&change.new_arguments, &change.old_arguments)
            };
            if &member.mod_args == target {
                continue;
            }
            if &member.mod_args != expected {
                return Err(crate::error::AppError::ConfigWriteError(format!(
                    "启动方案 {} 的账号 {} 已被其他操作修改，停止 Mod 引用事务",
                    change.group_id, change.account_id
                )));
            }
            member.mod_args = target.clone();
            changed = true;
        }
        Ok(changed)
    })
    .map_err(|error| error.to_string())?;
    Ok(())
}
