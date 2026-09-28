//! Durable same-name Mod replacement and crash recovery.
//!
//! This adapter owns its journal format and transaction paths. It only replaces
//! validated trees and retains the established backup/quarantine safeguards.
use super::filesystem::{
    canonical_safe_mods_root, ensure_safe_directory_if_present, ensure_safe_existing_node,
    path_exists_no_follow, remove_transaction_directory, rename_directory_and_sync, sync_directory,
    sync_safe_directory_tree, validate_safe_directory_tree,
};
use super::validation::{
    validate_audio_mod_directory, validate_recoverable_audio_mod_directory,
    validate_recoverable_backup_directory, validate_required_feature_groups_directory,
};
use crate::domain::mod_arguments::plain_mod_name;
use crate::domain::mod_processing::{
    validate_feature_group_entries, validate_preserved_feature_groups, GeneratorFeatureGroup,
};
use crate::infrastructure::durable_fs;
use crate::rune_audio::protocol::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Component, Path, PathBuf},
};

const REPLACE_JOURNAL_FORMAT_VERSION: u8 = 1;
pub(super) const REPLACE_JOURNAL_PREFIX: &str = ".d2rhub-audio-replace-";
pub(super) const REPLACE_JOURNAL_SUFFIX: &str = ".json";

#[derive(Debug, Deserialize, Serialize)]
struct AudioModReplaceJournal {
    format_version: u8,
    mod_name: String,
    staged_relative: PathBuf,
    backup_relative: PathBuf,
    #[serde(default)]
    required_feature_groups: Vec<GeneratorFeatureGroup>,
}

fn is_safe_relative_path(path: &Path) -> bool {
    path.components().next().is_some()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn valid_transaction_id(value: &str) -> bool {
    value.len() == 32
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && uuid::Uuid::parse_str(value).is_ok()
}

fn replacement_transaction_id(staged_relative: &Path, backup_relative: &Path) -> Option<String> {
    let stage_parent =
        staged_relative
            .components()
            .next()
            .and_then(|component| match component {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })?;
    let backup = backup_relative.file_name()?.to_str()?;
    let stage_id = stage_parent.strip_prefix(".d2rhub-upgrade-stage-")?;
    let backup_id = backup.strip_prefix(".d2rhub-upgrade-backup-")?;
    (stage_id == backup_id && valid_transaction_id(stage_id)).then(|| stage_id.to_string())
}

pub(super) fn replace_journal_paths_are_valid(
    mod_name: &str,
    staged_relative: &Path,
    backup_relative: &Path,
) -> bool {
    if !is_safe_relative_path(staged_relative)
        || !is_safe_relative_path(backup_relative)
        || staged_relative.components().count() != 2
        || backup_relative.components().count() != 1
        || staged_relative.file_name().and_then(|name| name.to_str()) != Some(mod_name)
    {
        return false;
    }
    replacement_transaction_id(staged_relative, backup_relative).is_some()
}

fn journal_transaction_id(journal_path: &Path) -> Option<String> {
    let name = journal_path.file_name()?.to_str()?;
    let id = name
        .strip_prefix(REPLACE_JOURNAL_PREFIX)?
        .strip_suffix(REPLACE_JOURNAL_SUFFIX)?;
    valid_transaction_id(id).then(|| id.to_string())
}

fn ensure_transaction_paths_safe(
    mods_directory: &Path,
    target_directory: &Path,
    staged_directory: &Path,
    backup_directory: &Path,
) -> Result<PathBuf, String> {
    let canonical_mods = canonical_safe_mods_root(mods_directory)?;
    ensure_safe_directory_if_present(&canonical_mods, target_directory, "更新目标")?;
    ensure_safe_directory_if_present(&canonical_mods, backup_directory, "更新备份")?;
    let stage_parent = staged_directory
        .parent()
        .ok_or_else(|| "更新暂存目录缺少父目录".to_string())?;
    ensure_safe_directory_if_present(&canonical_mods, stage_parent, "更新暂存父目录")?;
    ensure_safe_directory_if_present(&canonical_mods, staged_directory, "更新暂存目录")?;
    Ok(canonical_mods)
}

fn remove_replace_journal(mods_directory: &Path, journal_path: &Path) -> Result<(), String> {
    let canonical_mods = canonical_safe_mods_root(mods_directory)?;
    ensure_safe_existing_node(&canonical_mods, journal_path, false, "Mod 更新事务记录")?;
    std::fs::remove_file(journal_path)
        .map_err(|error| format!("清理 Mod 更新事务记录失败：{error}"))?;
    sync_directory(mods_directory)
}

pub(super) fn write_replace_journal(
    mods_directory: &Path,
    mod_name: &str,
    staged_directory: &Path,
    backup_directory: &Path,
    required_feature_groups: &[GeneratorFeatureGroup],
) -> Result<PathBuf, String> {
    write_replace_journal_with_stage_sync(
        mods_directory,
        mod_name,
        staged_directory,
        backup_directory,
        required_feature_groups,
        sync_safe_directory_tree,
    )
}

pub(super) fn write_replace_journal_with_stage_sync<F>(
    mods_directory: &Path,
    mod_name: &str,
    staged_directory: &Path,
    backup_directory: &Path,
    required_feature_groups: &[GeneratorFeatureGroup],
    sync_staged_tree: F,
) -> Result<PathBuf, String>
where
    F: FnOnce(&Path, &Path) -> Result<(), String>,
{
    validate_feature_group_entries(required_feature_groups, PROTOCOL_VERSION)
        .map_err(|error| format!("同名更新需要保留的功能组无效：{error}"))?;
    let staged_relative = staged_directory
        .strip_prefix(mods_directory)
        .map_err(|_| "同名更新的暂存目录必须位于 mods 目录内".to_string())?
        .to_path_buf();
    let backup_relative = backup_directory
        .strip_prefix(mods_directory)
        .map_err(|_| "同名更新的备份目录必须位于 mods 目录内".to_string())?
        .to_path_buf();
    if !replace_journal_paths_are_valid(mod_name, &staged_relative, &backup_relative) {
        return Err("同名更新的事务目录无效".to_string());
    }

    let transaction_id = replacement_transaction_id(&staged_relative, &backup_relative)
        .ok_or_else(|| "同名更新的事务标识无效".to_string())?;
    let journal_path = mods_directory.join(format!(
        "{REPLACE_JOURNAL_PREFIX}{transaction_id}{REPLACE_JOURNAL_SUFFIX}"
    ));
    let temporary_path = mods_directory.join(format!(
        "{REPLACE_JOURNAL_PREFIX}{transaction_id}{REPLACE_JOURNAL_SUFFIX}.tmp"
    ));
    let target_directory = mods_directory.join(mod_name);
    let canonical_mods = ensure_transaction_paths_safe(
        mods_directory,
        &target_directory,
        staged_directory,
        backup_directory,
    )?;
    ensure_safe_existing_node(&canonical_mods, staged_directory, true, "更新暂存目录")?;
    ensure_safe_existing_node(&canonical_mods, &target_directory, true, "更新目标")?;
    if path_exists_no_follow(&journal_path)? || path_exists_no_follow(&temporary_path)? {
        return Err("同名更新的事务记录发生冲突，请重试".to_string());
    }
    // The journal authorizes deletion of the last known-good backup later. It must never become
    // visible until every staged file and directory entry is durable and the no-link tree has been
    // revalidated immediately before the journal mutation.
    sync_staged_tree(mods_directory, staged_directory)
        .map_err(|error| format!("无法持久化同名更新的暂存 Mod：{error}"))?;
    validate_safe_directory_tree(mods_directory, staged_directory)
        .map_err(|error| format!("暂存 Mod 在写入事务记录前发生安全变化：{error}"))?;
    let staged_validation =
        validate_audio_mod_directory(mods_directory, mod_name, staged_directory.to_path_buf())
            .map_err(|error| format!("暂存 Mod 在写入事务记录前严格校验失败：{error}"))?;
    validate_preserved_feature_groups(required_feature_groups, &staged_validation.feature_groups)?;
    let journal = AudioModReplaceJournal {
        format_version: REPLACE_JOURNAL_FORMAT_VERSION,
        mod_name: mod_name.to_string(),
        staged_relative,
        backup_relative,
        required_feature_groups: required_feature_groups.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&journal)
        .map_err(|error| format!("无法创建 Mod 更新事务记录：{error}"))?;
    let write_result = (|| -> Result<(), String> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| format!("无法创建 Mod 更新事务记录：{error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("无法写入 Mod 更新事务记录：{error}"))?;
        file.sync_all()
            .map_err(|error| format!("无法持久化 Mod 更新事务记录：{error}"))?;
        durable_fs::durable_rename(&temporary_path, &journal_path)
            .map_err(|error| format!("无法提交 Mod 更新事务记录：{error}"))?;
        sync_directory(mods_directory)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&temporary_path);
    }
    write_result.map(|()| journal_path)
}

fn cleanup_staged_directory(mods_directory: &Path, staged_directory: &Path) -> Result<(), String> {
    let stage_parent = staged_directory
        .parent()
        .ok_or_else(|| "更新暂存目录缺少父目录".to_string())?;
    if !path_exists_no_follow(stage_parent)? {
        return Ok(());
    }
    if stage_parent.parent() != Some(mods_directory)
        || !stage_parent
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(".d2rhub-upgrade-stage-"))
    {
        return Err("拒绝清理事务范围外的 Mod 更新暂存目录".to_string());
    }
    remove_transaction_directory(mods_directory, stage_parent, "Mod 更新暂存目录")
}

fn ensure_rollback_marker(mods_directory: &Path, staged_directory: &Path) -> Result<(), String> {
    if path_exists_no_follow(staged_directory)? {
        return Ok(());
    }
    let stage_parent = staged_directory
        .parent()
        .ok_or_else(|| "更新暂存目录缺少父目录".to_string())?;
    if !path_exists_no_follow(stage_parent)? {
        std::fs::create_dir(stage_parent)
            .map_err(|error| format!("创建回滚标记目录失败：{error}"))?;
        sync_directory(mods_directory)?;
    }
    let canonical_mods = canonical_safe_mods_root(mods_directory)?;
    ensure_safe_existing_node(&canonical_mods, stage_parent, true, "更新暂存父目录")?;
    std::fs::create_dir(staged_directory).map_err(|error| format!("创建回滚标记失败：{error}"))?;
    sync_directory(stage_parent)?;
    sync_directory(mods_directory)
}

fn quarantine_directory(
    mods_directory: &Path,
    path: &Path,
    mod_name: &str,
    kind: &str,
) -> Result<PathBuf, String> {
    let quarantine = mods_directory.join(format!(
        ".d2rhub-upgrade-failed-{kind}-{}-{mod_name}",
        uuid::Uuid::new_v4().simple()
    ));
    rename_directory_and_sync(mods_directory, path, &quarantine, "隔离损坏的 Mod")?;
    Ok(quarantine)
}

pub(super) fn recover_audio_mod_replacements(mods_directory: &Path) -> Result<(), String> {
    let mut journal_paths = std::fs::read_dir(mods_directory)
        .map_err(|error| format!("无法检查 Mod 更新事务：{error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            (name.starts_with(REPLACE_JOURNAL_PREFIX) && name.ends_with(REPLACE_JOURNAL_SUFFIX))
                .then(|| entry.path())
        })
        .collect::<Vec<_>>();
    journal_paths.sort();

    for journal_path in journal_paths {
        let canonical_mods = canonical_safe_mods_root(mods_directory)?;
        ensure_safe_existing_node(&canonical_mods, &journal_path, false, "Mod 更新事务记录")?;
        let journal: AudioModReplaceJournal = serde_json::from_slice(
            &std::fs::read(&journal_path)
                .map_err(|error| format!("无法读取 Mod 更新事务记录：{error}"))?,
        )
        .map_err(|error| format!("Mod 更新事务记录已损坏：{error}"))?;
        let mod_name = plain_mod_name(&journal.mod_name)?;
        let transaction_id =
            replacement_transaction_id(&journal.staged_relative, &journal.backup_relative);
        if journal.format_version != REPLACE_JOURNAL_FORMAT_VERSION
            || !replace_journal_paths_are_valid(
                mod_name,
                &journal.staged_relative,
                &journal.backup_relative,
            )
            || transaction_id != journal_transaction_id(&journal_path)
        {
            return Err("Mod 更新事务记录版本或路径无效，已停止自动恢复".to_string());
        }
        validate_feature_group_entries(&journal.required_feature_groups, PROTOCOL_VERSION)
            .map_err(|error| format!("Mod 更新事务需要保留的功能组无效：{error}"))?;
        let target_directory = mods_directory.join(mod_name);
        let staged_directory = mods_directory.join(&journal.staged_relative);
        let backup_directory = mods_directory.join(&journal.backup_relative);
        ensure_transaction_paths_safe(
            mods_directory,
            &target_directory,
            &staged_directory,
            &backup_directory,
        )?;
        let target_exists = path_exists_no_follow(&target_directory)?;
        let staged_exists = path_exists_no_follow(&staged_directory)?;
        let backup_exists = path_exists_no_follow(&backup_directory)?;
        if target_exists && staged_exists && backup_exists {
            return Err(
                "Mod 更新事务同时存在目标、暂存和备份，状态不明确，已停止自动恢复".to_string(),
            );
        }

        let mut final_is_strict = true;
        let mut quarantine_bad_backup = false;
        match (target_exists, staged_exists, backup_exists) {
            // Journal persisted, but the first switch never happened. This is the only target state
            // that may be an old published Mod and therefore uses the compatibility validator.
            (true, true, false) => {
                validate_recoverable_audio_mod_directory(
                    mods_directory,
                    mod_name,
                    &target_directory,
                )
                .map_err(|error| format!("未切换的旧版 Mod 无法通过恢复校验：{error}"))?;
                final_is_strict = false;
            }
            // Backup proves the staged directory was already switched into the target. Never accept
            // that new target via the permissive legacy validator.
            (true, false, true) => {
                if let Err(target_error) = validate_required_feature_groups_directory(
                    mods_directory,
                    mod_name,
                    target_directory.clone(),
                    &journal.required_feature_groups,
                ) {
                    validate_recoverable_backup_directory(
                        mods_directory,
                        mod_name,
                        &backup_directory,
                        &journal.required_feature_groups,
                    )
                    .map_err(|backup_error| {
                        format!(
                            "新版与备份 Mod 都无法通过恢复校验；新版：{target_error}；备份：{backup_error}"
                        )
                    })?;
                    // Keep the rejected new target in the staged slot. Besides preserving evidence,
                    // this makes a crash after rollback unambiguously recover as an old target.
                    let stage_parent = staged_directory
                        .parent()
                        .ok_or_else(|| "更新暂存目录缺少父目录".to_string())?;
                    if !path_exists_no_follow(stage_parent)? {
                        std::fs::create_dir(stage_parent)
                            .map_err(|error| format!("创建损坏新版隔离目录失败：{error}"))?;
                        sync_directory(mods_directory)?;
                    }
                    rename_directory_and_sync(
                        mods_directory,
                        &target_directory,
                        &staged_directory,
                        "隔离损坏的新版 Mod",
                    )?;
                    rename_directory_and_sync(
                        mods_directory,
                        &backup_directory,
                        &target_directory,
                        "恢复旧版 Mod",
                    )?;
                    final_is_strict = false;
                }
            }
            // With no remaining transaction artifacts, the target can only be the newly installed
            // candidate. A legacy/recoverable check here could bless a corrupted new target.
            (true, false, false) => {}
            (false, staged, true) => {
                let backup_validation = validate_recoverable_backup_directory(
                    mods_directory,
                    mod_name,
                    &backup_directory,
                    &journal.required_feature_groups,
                );
                match backup_validation {
                    Ok(()) => {
                        if !staged {
                            ensure_rollback_marker(mods_directory, &staged_directory)?;
                        }
                        rename_directory_and_sync(
                            mods_directory,
                            &backup_directory,
                            &target_directory,
                            "恢复旧版 Mod",
                        )?;
                        final_is_strict = false;
                    }
                    Err(backup_error) if staged => {
                        let staged_validation = validate_audio_mod_directory(
                            mods_directory,
                            mod_name,
                            staged_directory.clone(),
                        )
                        .map_err(|staged_error| {
                            format!(
                                "备份与暂存 Mod 都无法恢复；备份：{backup_error}；暂存：{staged_error}"
                            )
                        })?;
                        validate_preserved_feature_groups(
                            &journal.required_feature_groups,
                            &staged_validation.feature_groups,
                        )?;
                        sync_safe_directory_tree(mods_directory, &staged_directory)?;
                        validate_required_feature_groups_directory(
                            mods_directory,
                            mod_name,
                            staged_directory.clone(),
                            &journal.required_feature_groups,
                        )
                        .map_err(|error| format!("暂存 Mod 在安装前发生安全变化：{error}"))?;
                        rename_directory_and_sync(
                            mods_directory,
                            &staged_directory,
                            &target_directory,
                            "完成暂存 Mod 安装",
                        )?;
                        quarantine_bad_backup = true;
                    }
                    Err(backup_error) => {
                        return Err(format!(
                            "更新备份无法通过恢复校验，且没有严格有效的暂存 Mod：{backup_error}"
                        ));
                    }
                }
            }
            (false, true, false) => {
                let staged_validation = validate_audio_mod_directory(
                    mods_directory,
                    mod_name,
                    staged_directory.clone(),
                )
                .map_err(|error| format!("更新暂存 Mod 无法恢复：{error}"))?;
                validate_preserved_feature_groups(
                    &journal.required_feature_groups,
                    &staged_validation.feature_groups,
                )?;
                sync_safe_directory_tree(mods_directory, &staged_directory)?;
                validate_required_feature_groups_directory(
                    mods_directory,
                    mod_name,
                    staged_directory.clone(),
                    &journal.required_feature_groups,
                )
                .map_err(|error| format!("暂存 Mod 在安装前发生安全变化：{error}"))?;
                rename_directory_and_sync(
                    mods_directory,
                    &staged_directory,
                    &target_directory,
                    "完成暂存 Mod 安装",
                )?;
            }
            (false, false, false) => {
                return Err("Mod 更新事务缺少目标、备份与暂存目录，无法自动恢复".to_string())
            }
            (true, true, true) => unreachable!("ambiguous state was rejected above"),
        }

        if final_is_strict {
            validate_required_feature_groups_directory(
                mods_directory,
                mod_name,
                target_directory.clone(),
                &journal.required_feature_groups,
            )
            .map_err(|error| format!("新版 Mod 恢复后严格校验失败：{error}"))?;
            sync_safe_directory_tree(mods_directory, &target_directory)
                .map_err(|error| format!("新版 Mod 恢复后持久化失败：{error}"))?;
            validate_required_feature_groups_directory(
                mods_directory,
                mod_name,
                target_directory.clone(),
                &journal.required_feature_groups,
            )
            .map_err(|error| format!("新版 Mod 在清理备份前发生安全变化：{error}"))?;
            if path_exists_no_follow(&backup_directory)? {
                if quarantine_bad_backup {
                    let _ = quarantine_directory(
                        mods_directory,
                        &backup_directory,
                        mod_name,
                        "backup",
                    )?;
                } else {
                    remove_transaction_directory(
                        mods_directory,
                        &backup_directory,
                        "Mod 更新备份",
                    )?;
                }
            }
            cleanup_staged_directory(mods_directory, &staged_directory)?;
            remove_replace_journal(mods_directory, &journal_path)?;
        } else {
            if journal.required_feature_groups.is_empty() {
                validate_recoverable_audio_mod_directory(
                    mods_directory,
                    mod_name,
                    &target_directory,
                )
                .map_err(|error| format!("旧版 Mod 恢复后校验失败：{error}"))?;
            } else {
                let restored = validate_audio_mod_directory(
                    mods_directory,
                    mod_name,
                    target_directory.clone(),
                )
                .map_err(|error| format!("原功能组 Mod 恢复后严格校验失败：{error}"))?;
                validate_preserved_feature_groups(
                    &journal.required_feature_groups,
                    &restored.feature_groups,
                )?;
            }
            // Keep the staged path as a rollback marker until the journal is durably removed. If
            // cleanup or journal deletion is interrupted, target+staging still selects legacy-safe
            // validation on the next launch rather than misclassifying the old target as new.
            ensure_rollback_marker(mods_directory, &staged_directory)?;
            remove_replace_journal(mods_directory, &journal_path)?;
            cleanup_staged_directory(mods_directory, &staged_directory)?;
        }
    }
    Ok(())
}

pub(super) fn replace_audio_mod_directory(
    mods_directory: &Path,
    mod_name: &str,
    staged_directory: &Path,
    backup_directory: &Path,
    required_feature_groups: &[GeneratorFeatureGroup],
) -> Result<(), String> {
    recover_audio_mod_replacements(mods_directory)?;
    let target_directory = mods_directory.join(mod_name);
    let staged_relative = staged_directory
        .strip_prefix(mods_directory)
        .map_err(|_| "同名更新的暂存目录必须位于 mods 目录内".to_string())?;
    let backup_relative = backup_directory
        .strip_prefix(mods_directory)
        .map_err(|_| "同名更新的备份目录必须位于 mods 目录内".to_string())?;
    if !replace_journal_paths_are_valid(mod_name, staged_relative, backup_relative) {
        return Err("同名更新的事务目录无效".to_string());
    }
    ensure_transaction_paths_safe(
        mods_directory,
        &target_directory,
        staged_directory,
        backup_directory,
    )?;
    if !path_exists_no_follow(staged_directory)? {
        return Err("同名更新的暂存目录不存在".to_string());
    }
    if !path_exists_no_follow(&target_directory)? {
        return Err(format!("待更新的 Mod 不存在：{mod_name}"));
    }
    if path_exists_no_follow(backup_directory)? {
        return Err("同名更新的备份目录发生冲突，请重试".to_string());
    }
    let staged_validation =
        validate_audio_mod_directory(mods_directory, mod_name, staged_directory.to_path_buf())
            .map_err(|error| format!("同名更新的暂存 Mod 严格校验失败：{error}"))?;
    validate_preserved_feature_groups(required_feature_groups, &staged_validation.feature_groups)?;
    validate_recoverable_audio_mod_directory(mods_directory, mod_name, &target_directory)
        .map_err(|error| format!("同名更新的旧版 Mod 无法安全备份：{error}"))?;

    let journal_path = match write_replace_journal(
        mods_directory,
        mod_name,
        staged_directory,
        backup_directory,
        required_feature_groups,
    ) {
        Ok(path) => path,
        Err(error) => {
            // A directory-sync error can happen after the named journal is already visible. Recover
            // it while the staged directory is still alive, so the caller's temporary-directory
            // guard cannot turn a prepared legacy target into an ambiguous target-only journal.
            let recovery = recover_audio_mod_replacements(mods_directory);
            return Err(match recovery {
                Ok(()) => error,
                Err(recovery_error) => {
                    format!("{error}；同时清理未提交事务失败：{recovery_error}")
                }
            });
        }
    };
    let staged_validation =
        validate_audio_mod_directory(mods_directory, mod_name, staged_directory.to_path_buf())
            .map_err(|error| format!("暂存 Mod 在目录切换前发生安全变化：{error}"))?;
    validate_preserved_feature_groups(required_feature_groups, &staged_validation.feature_groups)?;
    if let Err(error) = rename_directory_and_sync(
        mods_directory,
        &target_directory,
        backup_directory,
        "备份旧 Mod",
    ) {
        let recovery = recover_audio_mod_replacements(mods_directory);
        return Err(match recovery {
            Ok(()) => format!("无法备份旧 Mod；请确认游戏已经关闭：{error}"),
            Err(recovery_error) => {
                format!("无法备份旧 Mod，且事务恢复需要人工处理：{error}；{recovery_error}")
            }
        });
    }
    if let Err(error) =
        validate_audio_mod_directory(mods_directory, mod_name, staged_directory.to_path_buf())
            .and_then(|validated| {
                validate_preserved_feature_groups(
                    required_feature_groups,
                    &validated.feature_groups,
                )
            })
    {
        let recovery = recover_audio_mod_replacements(mods_directory);
        return Err(match recovery {
            Ok(()) => format!("暂存 Mod 在安装前发生安全变化，已恢复旧版：{error}"),
            Err(recovery_error) => format!(
                "暂存 Mod 在安装前发生安全变化，且事务恢复需要人工处理：{error}；{recovery_error}"
            ),
        });
    }
    match rename_directory_and_sync(
        mods_directory,
        staged_directory,
        &target_directory,
        "安装新版 Mod",
    ) {
        Ok(()) => {
            if let Err(validation_error) =
                validate_audio_mod_directory(mods_directory, mod_name, target_directory.clone())
                    .and_then(|validated| {
                        validate_preserved_feature_groups(
                            required_feature_groups,
                            &validated.feature_groups,
                        )
                    })
            {
                let recovery = recover_audio_mod_replacements(mods_directory);
                return Err(match recovery {
                    Ok(()) => {
                        format!("安装后的新版 Mod 严格校验失败，已恢复旧版：{validation_error}")
                    }
                    Err(recovery_error) => format!(
                        "安装后的新版 Mod 严格校验失败，且事务恢复需要人工处理：{validation_error}；{recovery_error}"
                    ),
                });
            }
            sync_safe_directory_tree(mods_directory, &target_directory)
                .map_err(|error| format!("安装后的新版 Mod 持久化失败：{error}"))?;
            validate_required_feature_groups_directory(
                mods_directory,
                mod_name,
                target_directory.clone(),
                required_feature_groups,
            )
            .map_err(|error| format!("新版 Mod 在清理备份前发生安全变化：{error}"))?;
            remove_transaction_directory(mods_directory, backup_directory, "Mod 更新备份")?;
            cleanup_staged_directory(mods_directory, staged_directory)?;
            remove_replace_journal(mods_directory, &journal_path)?;
            Ok(())
        }
        Err(install_error) => {
            let recovery = recover_audio_mod_replacements(mods_directory);
            Err(match recovery {
                Ok(()) => format!("安装新版 Mod 失败，已恢复旧版：{install_error}"),
                Err(recovery_error) => format!(
                    "安装新版 Mod 失败且自动恢复未完成；旧版仍保留在 {}。安装错误：{}；恢复错误：{}",
                    backup_directory.display(),
                    install_error,
                    recovery_error
                ),
            })
        }
    }
}
