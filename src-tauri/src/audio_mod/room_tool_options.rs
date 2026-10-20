//! Processing preferences applied to generated output before installation.
use std::path::Path;

const MANIFESTS: [&str; 2] = ["d2rhub-mod-manifest.json", "audio-telemetry-manifest.json"];
const OPTION: &str = "room_tools_default_hell";

pub(super) fn default_hell(directory: &Path) -> Result<bool, String> {
    let Some(path) = MANIFESTS
        .iter()
        .map(|name| directory.join(name))
        .find(|p| p.is_file())
    else {
        return Ok(true);
    };
    let document: serde_json::Value = serde_json::from_slice(
        &std::fs::read(path).map_err(|e| format!("读取房间工具加工选项失败：{e}"))?,
    )
    .map_err(|e| format!("房间工具加工选项损坏：{e}"))?;
    match document.get(OPTION) {
        None => Ok(true),
        Some(serde_json::Value::Bool(enabled)) => Ok(*enabled),
        _ => Err("创建房间默认地狱的加工选项无效".into()),
    }
}

pub(super) fn configure(output: &Path, name: &str, enabled: bool) -> Result<(), String> {
    crate::domain::mod_arguments::plain_mod_name(name)?;
    let directory = output.join(name);
    super::filesystem::validate_safe_directory_tree(output, &directory)?;
    let paths: Vec<_> = MANIFESTS
        .iter()
        .map(|file| directory.join(file))
        .filter(|p| p.is_file())
        .collect();
    let mut manifests = Vec::new();
    for path in paths {
        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        manifests.push((path, document));
    }
    if !manifests.iter().any(|(_, doc)| {
        doc["feature_groups"].as_array().is_some_and(|groups| {
            groups
                .iter()
                .any(|group| group["id"] == "in_game_room_tools")
        })
    }) {
        return Ok(());
    }
    let path = directory
        .join(format!("{name}.mpq"))
        .join(super::validation::layouts::ROOM_TOOL_LAYOUT_DIRECTORY)
        .join("D2RHubInGameCreateGamehd.json");
    let mut form: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).map_err(|e| format!("读取局内创建表单失败：{e}"))?,
    )
    .map_err(|e| e.to_string())?;
    let children = form["children"]
        .as_array_mut()
        .ok_or("局内创建表单缺少 children")?;
    children.retain(|child| child["name"] != "D2RHubDefaultHell");
    if enabled {
        children.push(serde_json::json!({
            "type": "TimerWidget", "name": "D2RHubDefaultHell",
            "fields": {"time": 0.05, "message": "CreateGame:SetDifficulty:2"}
        }));
    }
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&form).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    for (path, mut document) in manifests {
        document[OPTION] = enabled.into();
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&document).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
