//! Atomic edits to an installed feature, with content validation and rollback.
use super::filesystem::{canonical_safe_mods_root, ensure_safe_existing_node};
use super::validation::{
    layouts::{validate_auto_exit_on_death_layouts, ROOM_TOOL_LAYOUT_DIRECTORY},
    validate_audio_mod_credential,
};
use crate::domain::mod_processing::AUTO_EXIT_ON_DEATH_FEATURE_ID;
use crate::infrastructure::durable_fs;
use std::{io::Write, path::Path};

fn replace_mod_layout_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Mod 布局文件名无效".to_string())?;
    let temporary = path.with_file_name(format!(
        ".{file_name}.d2rhub-toggle-{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let result = (|| -> Result<(), String> {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("无法创建 Mod 配置临时文件：{error}"))?;
        file.write_all(contents)
            .map_err(|error| format!("无法写入 Mod 配置：{error}"))?;
        file.sync_all()
            .map_err(|error| format!("无法持久化 Mod 配置：{error}"))?;
        drop(file);
        durable_fs::durable_sibling_replace(&temporary, path)
            .map_err(|error| format!("无法原子切换 Mod 配置：{error}"))?;
        if let Some(parent) = path.parent() {
            durable_fs::sync_directory(parent)
                .map_err(|error| format!("无法持久化 Mod 配置目录：{error}"))?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

pub(super) fn set_auto_exit_on_death_enabled(
    mods_directory: &Path,
    mod_name: &str,
    enabled: bool,
) -> Result<bool, String> {
    let validated = validate_audio_mod_credential(mods_directory, mod_name)?;
    if !validated
        .feature_groups
        .iter()
        .any(|group| group.id == AUTO_EXIT_ON_DEATH_FEATURE_ID)
    {
        return Err(format!("Mod“{mod_name}”不支持死亡后自动退房"));
    }
    if validated.auto_exit_on_death_enabled == enabled {
        return Ok(enabled);
    }
    validate_auto_exit_on_death_layouts(
        &validated.directory,
        mod_name,
        validated.auto_exit_on_death_enabled,
    )?;

    let layout_path = validated
        .directory
        .join(format!("{mod_name}.mpq"))
        .join(ROOM_TOOL_LAYOUT_DIRECTORY)
        .join("youdiedmodalhd.json");
    let canonical_mods = canonical_safe_mods_root(mods_directory)?;
    ensure_safe_existing_node(&canonical_mods, &layout_path, false, "死亡界面布局")?;
    let original =
        std::fs::read(&layout_path).map_err(|error| format!("无法读取死亡界面布局：{error}"))?;
    let mut document: serde_json::Value = serde_json::from_slice(&original)
        .map_err(|_| "死亡界面布局已损坏，无法切换死亡退房".to_string())?;
    let children = document
        .get_mut("children")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| "死亡界面布局缺少 children，无法切换死亡退房".to_string())?;
    children.retain(|child| {
        child.get("name").and_then(serde_json::Value::as_str)
            != Some("D2RHubAutoExitOnDeathLauncher")
    });
    if enabled {
        children.push(serde_json::json!({
            "type": "TimerWidget",
            "name": "D2RHubAutoExitOnDeathLauncher",
            "fields": {
                "time": 0.01,
                "message": "PanelManager:OpenPanel:D2RHubAutoExitOnDeath"
            }
        }));
    }
    let updated = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("无法序列化死亡退房配置：{error}"))?;
    replace_mod_layout_file(&layout_path, &updated)?;

    match validate_audio_mod_credential(mods_directory, mod_name).and_then(|after| {
        validate_auto_exit_on_death_layouts(&after.directory, mod_name, enabled)?;
        Ok(after)
    }) {
        Ok(after) if after.auto_exit_on_death_enabled == enabled => Ok(enabled),
        validation => {
            let restore = replace_mod_layout_file(&layout_path, &original);
            let detail = match validation {
                Ok(_) => "写入后的启用状态与请求不一致".to_string(),
                Err(error) => error,
            };
            match restore {
                Ok(()) => Err(format!("死亡退房配置校验失败，已恢复原配置：{detail}")),
                Err(error) => Err(format!(
                    "死亡退房配置校验失败且自动恢复失败：{detail}；恢复错误：{error}"
                )),
            }
        }
    }
}
