//! Generate shared lightweight Mods without an account or audio feature dependency.
use crate::application::task_runtime::{TaskHandle, TaskRequest};
use crate::audio_mod::BuildLease;
use crate::state::SharedState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

const MANIFEST: &str = "generation-manifest.json";
const PRODUCER: &str = "d2r-native-bundled-generator";
pub const TASK_KIND: &str = "lightweight-mod-generate";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub edition: String,
    pub profile: String,
    pub mod_name: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct GenerateResult {
    pub edition: String,
    pub profile: String,
    pub mod_name: String,
    pub mod_directory: String,
    pub launch_arguments: String,
    pub task_id: u64,
}
#[derive(Serialize)]
pub struct GenerateContext {
    pub edition: String,
    pub game_directory: String,
    pub available: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct Metadata {
    pub profile: String,
    pub arguments: String,
}
pub(crate) fn arguments(profile: &str, name: &str) -> Result<String, String> {
    generator_arguments(profile, name)?;
    Ok(format!("-mod {name} -txt -assettestmode 1"))
}
// Preserve compatibility with manifests written before the Hub launch default changed.
fn generator_arguments(profile: &str, name: &str) -> Result<String, String> {
    if !matches!(profile, "main" | "filler" | "min") {
        return Err("未知轻量方案".into());
    }
    validate_name(name)?;
    Ok(format!(
        "-mod {name}{}",
        if profile == "main" { " -txt" } else { "" }
    ))
}
fn validate_name(name: &str) -> Result<(), String> {
    let upper = name.to_ascii_uppercase();
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
    {
        return Err("Mod 名称只允许英文字母、数字、短横线和下划线，最长 64 字符".into());
    }
    Ok(())
}
fn edition(value: &str) -> Result<&'static str, String> {
    match value {
        "CN" => Ok("CN"),
        "Global" => Ok("Global"),
        _ => Err("请选择国服或国际服".into()),
    }
}
pub(crate) fn game_path(state: &SharedState, value: &str) -> Result<PathBuf, String> {
    let config = state.configuration().snapshot().ok_or("尚未配置游戏目录")?;
    let path = if edition(value)? == "CN" {
        &config.cn_game_path
    } else {
        &config.global_game_path
    };
    if path.trim().is_empty() {
        return Err("请先在运行环境中配置该版本的游戏目录".into());
    }
    let root = PathBuf::from(path.trim());
    if !root.join(".build.info").is_file() || !root.join("Data").is_dir() {
        return Err("游戏目录缺少原版 Data 或版本信息，请检查运行环境设置".into());
    }
    root.canonicalize().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn get_lightweight_mod_context(
    state: tauri::State<'_, SharedState>,
    edition: String,
) -> Result<GenerateContext, String> {
    let normalized = self::edition(&edition)?.to_string();
    Ok(match game_path(state.inner(), &normalized) {
        Ok(path) => GenerateContext {
            edition: normalized,
            game_directory: path.to_string_lossy().into_owned(),
            available: true,
            reason: None,
        },
        Err(reason) => GenerateContext {
            edition: normalized,
            game_directory: String::new(),
            available: false,
            reason: Some(reason),
        },
    })
}
fn regular(path: &Path) -> Result<fs::Metadata, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("生成结果包含重解析点".into());
        }
    }
    if metadata.file_type().is_symlink() {
        return Err("生成结果包含符号链接".into());
    }
    Ok(metadata)
}
fn read_report(root: &Path) -> Result<Value, String> {
    regular(root)?;
    let path = root.join(MANIFEST);
    let m = regular(&path)?;
    if !m.is_file() || m.len() > 32 * 1024 * 1024 {
        return Err("生成清单大小或类型无效".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("生成清单损坏：{e}"))
}
fn metadata(report: &Value, root: &Path, name: &str) -> Result<Metadata, String> {
    validate_name(name)?;
    let profile = report["profile"].as_str().ok_or("生成清单缺少方案")?;
    let args = arguments(profile, name)?;
    let legacy_args = generator_arguments(profile, name)?;
    if report["producer"] != PRODUCER
        || report["mode"] != "bundled_rebuild"
        || report["mod_name"] != name
        || (report["launch_arguments"] != args
            && report["launch_arguments"] != legacy_args
            && report["launch_arguments"] != format!("{legacy_args} -assettestmode 1"))
        || report["verified_output_integrity"] != true
    {
        return Err("轻量 Mod 清单与名称或启动参数不一致".into());
    }
    let mpq = root.join(format!("{name}.mpq"));
    regular(&mpq)?;
    let info_path = mpq.join("modinfo.json");
    if regular(&info_path)?.len() > 64 * 1024 {
        return Err("Mod 元数据过大".into());
    }
    let info: Value = serde_json::from_slice(&fs::read(info_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if info["name"] != name {
        return Err("Mod 元数据名称不一致".into());
    }
    let version_path = mpq.join("data/global/dataversionbuild.txt");
    // Reject links on all components of the version path, not just its leaf.
    regular(&mpq.join("data"))?;
    regular(&mpq.join("data/global"))?;
    if regular(&version_path)?.len() > 128 {
        return Err("数据版本文件过大".into());
    }
    let version = fs::read_to_string(version_path).map_err(|e| e.to_string())?;
    let version = version.trim_start_matches('\u{feff}').trim();
    if version.is_empty()
        || !version.bytes().all(|b| b.is_ascii_digit())
        || report["game_data_version"] != version
    {
        return Err("游戏数据版本与生成清单不一致".into());
    }
    Ok(Metadata {
        profile: profile.into(),
        arguments: args,
    })
}
pub(crate) fn inspect(root: &Path, name: &str) -> Result<Option<Metadata>, String> {
    if !root.join(MANIFEST).exists() {
        return Ok(None);
    }
    let report = read_report(root)?;
    if report["producer"] != PRODUCER {
        return Ok(None);
    }
    metadata(&report, root, name).map(Some)
}
fn tree_totals(root: &Path) -> Result<(u64, u64), String> {
    let m = regular(root)?;
    if m.is_file() {
        return Ok((1, m.len()));
    }
    if !m.is_dir() {
        return Err("生成结果包含特殊文件".into());
    }
    let mut total = (0, 0);
    for item in fs::read_dir(root).map_err(|e| e.to_string())? {
        let value = tree_totals(&item.map_err(|e| e.to_string())?.path())?;
        total.0 += value.0;
        total.1 += value.1;
    }
    Ok(total)
}
fn validate_output(
    root: &Path,
    request: &GenerateRequest,
    returned: &Value,
) -> Result<Value, String> {
    let report = read_report(root)?;
    if &report != returned {
        return Err("返回报告与磁盘清单不一致".into());
    }
    let info = metadata(&report, root, &request.mod_name)?;
    if info.profile != request.profile {
        return Err("生成结果方案不一致".into());
    }
    let returned_path = PathBuf::from(report["mod_directory"].as_str().ok_or("生成报告缺少目录")?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if returned_path != root.canonicalize().map_err(|e| e.to_string())? {
        return Err("生成器返回了任务目录之外的路径".into());
    }
    tree_totals(root)?;
    let (files, bytes) = tree_totals(&root.join(format!("{}.mpq", request.mod_name)))?;
    if report["counts"]["verified_files"].as_u64() != Some(files)
        || report["generated_bytes"].as_u64() != Some(bytes)
    {
        return Err("生成文件集合或体积与校验报告不一致".into());
    }
    Ok(report)
}
fn name_exists(root: &Path, name: &str) -> Result<bool, String> {
    if !root.exists() {
        return Ok(false);
    }
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        if entry
            .map_err(|e| e.to_string())?
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
        {
            return Ok(true);
        }
    }
    Ok(false)
}
/// Only our UUID-owned job directory is eligible for cleanup, after child exit.
fn cleanup(root: &Path, parent: &Path) -> Result<(), String> {
    let actual = root.canonicalize().map_err(|e| e.to_string())?;
    if actual.parent() != Some(parent)
        || !actual
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with(".d2rhub-lightweight-"))
    {
        return Err("拒绝清理任务目录之外的路径".into());
    }
    tree_totals(root)?;
    fs::remove_dir_all(root).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn generate_lightweight_mod(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    edition: String,
    profile: String,
    mod_name: String,
) -> Result<GenerateResult, String> {
    run_task(
        app,
        state,
        GenerateRequest {
            edition,
            profile,
            mod_name,
        },
        None,
    )
    .await
}
pub(crate) async fn retry(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    request: GenerateRequest,
    previous: u64,
) -> Result<(), String> {
    run_task(app, state, request, Some(previous))
        .await
        .map(|_| ())
}
async fn run_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    request: GenerateRequest,
    retry_of: Option<u64>,
) -> Result<GenerateResult, String> {
    edition(&request.edition)?;
    arguments(&request.profile, &request.mod_name)?;
    let shared = state.inner().clone();
    let _lease = BuildLease::acquire(&shared)?;
    let mut descriptor = TaskRequest::new(TASK_KIND)
        .for_subject(format!("{}:{}", request.edition, request.mod_name))
        .with_conflict_key("audio-mod-build")
        .with_retry_payload(serde_json::to_string(&request).map_err(|e| e.to_string())?)
        .with_initial_status("preflight", "正在检查轻量 Mod 生成环境");
    if let Some(id) = retry_of {
        descriptor = descriptor.with_retry_of(id);
    }
    let task = shared
        .tasks()
        .begin(descriptor)
        .map_err(|e| e.to_string())?;
    let result = generate_impl(&app, &shared, &request, &task).await;
    match &result {
        Ok(_) => {
            let _ = task.succeed("轻量 Mod 已生成，可分配给账号");
        }
        Err(e) if task.cancellation_requested() => {
            let _ = task.cancelled(e);
        }
        Err(e) => {
            let _ = task.fail("lightweight-generation-failed", e);
        }
    }
    result
}
async fn generate_impl(
    app: &tauri::AppHandle,
    state: &SharedState,
    request: &GenerateRequest,
    task: &TaskHandle,
) -> Result<GenerateResult, String> {
    let game = game_path(state, &request.edition)?;
    let mods = game.join("mods");
    fs::create_dir_all(&mods).map_err(|e| e.to_string())?;
    let mods = mods.canonicalize().map_err(|e| e.to_string())?;
    if name_exists(&mods, &request.mod_name)? {
        return Err("同名 Mod 已存在，请使用现有成品或修改名称".into());
    }
    let stage = mods.join(format!(".d2rhub-lightweight-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let command =
        crate::mod_resources::resolve_processor(app).map(|path| app.shell().command(path));
    let launched = command.and_then(|cmd| {
        cmd.args([
            "lightweight",
            "--game",
            &game.to_string_lossy(),
            "--profile",
            &request.profile,
            "--name",
            &request.mod_name,
            "--output",
            &stage.to_string_lossy(),
            "--events",
        ])
        .spawn()
        .map_err(|e| e.to_string())
    });
    let (mut events, child) = match launched {
        Ok(pair) => pair,
        Err(e) => {
            let _ = cleanup(&stage, &mods);
            return Err(format!("无法启动独立加工器：{e}"));
        }
    };
    let mut child = Some(child);
    let mut report = None;
    let mut error = String::new();
    let mut percent = 0u8;
    let mut cancelling = false;
    let mut deadline = None;
    let process_result = loop {
        if task.cancellation_requested() && !cancelling {
            if let Some(child) = child.take() {
                if let Err(e) = child.kill() {
                    return Err(format!(
                        "无法停止生成器，任务目录保留在 {}：{e}",
                        stage.display()
                    ));
                }
            }
            cancelling = true;
            deadline = Some(std::time::Instant::now() + Duration::from_secs(15));
        }
        if deadline.is_some_and(|d| std::time::Instant::now() > d) {
            return Err(format!(
                "等待生成器退出超时，任务目录保留在 {}",
                stage.display()
            ));
        }
        let event = match tokio::time::timeout(Duration::from_millis(100), events.recv()).await {
            Ok(Some(e)) => e,
            Ok(None) => break Err("生成器事件通道提前关闭".to_string()),
            Err(_) => continue,
        };
        match event {
            CommandEvent::Stdout(bytes) => {
                if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                    match value["type"].as_str() {
                        Some("progress") => {
                            percent = percent.max(
                                (value["percent"].as_u64().unwrap_or(0).min(100) * 90 / 100) as u8,
                            );
                            let _ = task.update(
                                percent,
                                "generating",
                                value["message"].as_str().unwrap_or("正在生成资源"),
                            );
                        }
                        Some("completed") => report = Some(value["report"].clone()),
                        Some("error") => {
                            error = value["message"].as_str().unwrap_or("生成失败").into()
                        }
                        _ => {}
                    }
                }
            }
            CommandEvent::Stderr(bytes) => {
                if error.len() < 16000 {
                    error.push_str(&String::from_utf8_lossy(&bytes));
                }
            }
            CommandEvent::Error(message) => {
                if error.len() < 16000 {
                    error.push_str(&message);
                }
            }
            CommandEvent::Terminated(payload) => {
                break if cancelling {
                    Err("已取消生成".into())
                } else if payload.code == Some(0) {
                    report.ok_or_else(|| "生成器未返回完整报告，请更新内置生成器".into())
                } else {
                    Err(if error.is_empty() {
                        format!("生成器退出：{:?}", payload.code)
                    } else {
                        error
                    })
                }
            }
            _ => {}
        }
    };
    // On channel loss there is no exit confirmation; retain the staging directory.
    if process_result
        .as_ref()
        .is_err_and(|e| e == "生成器事件通道提前关闭")
    {
        if let Some(child) = child.take() {
            let _ = child.kill();
        }
        return Err(format!(
            "生成器事件通道提前关闭，任务目录保留在 {}",
            stage.display()
        ));
    }
    let result = async {
        let returned = process_result?;
        let root = stage.join(&request.mod_name);
        let _ = task.update(92, "validate", "正在校验生成结果");
        let check_root = root.clone();
        let check_request = request.clone();
        let mut disk = tokio::task::spawn_blocking(move || {
            validate_output(&check_root, &check_request, &returned)
        })
        .await
        .map_err(|e| e.to_string())??;
        if task.cancellation_requested() {
            return Err("已取消生成".into());
        }
        if game_path(state, &request.edition)? != game {
            return Err("生成期间游戏目录已改变，请重新生成".into());
        }
        if name_exists(&mods, &request.mod_name)? {
            return Err("同名 Mod 在生成期间出现，未覆盖现有内容".into());
        }
        let destination = mods.join(&request.mod_name);
        disk["mod_directory"] = Value::String(destination.to_string_lossy().into_owned());
        fs::write(
            root.join(MANIFEST),
            serde_json::to_vec_pretty(&disk).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let _ = task.update(98, "install", "正在加入 Mod 列表");
        fs::rename(&root, &destination).map_err(|e| format!("安装失败，未覆盖现有 Mod：{e}"))?;
        Ok(GenerateResult {
            edition: request.edition.clone(),
            profile: request.profile.clone(),
            mod_name: request.mod_name.clone(),
            mod_directory: destination.to_string_lossy().into_owned(),
            launch_arguments: arguments(&request.profile, &request.mod_name)?,
            task_id: task.task_id(),
        })
    }
    .await;
    if let Err(e) = cleanup(&stage, &mods) {
        log::warn!("轻量 Mod 任务目录保留 {}: {e}", stage.display());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires an installed game and the bundled generator"]
    fn bundled_generator_outputs_pass_hub_validation() {
        let game = std::env::var("D2RHUB_LIGHTWEIGHT_GAME_ROOT").expect("game root");
        let parent =
            std::env::temp_dir().join(format!("hub-generator-smoke-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&parent).unwrap();
        for (profile, name) in [("main", "LiteHub"), ("filler", "BoHub"), ("min", "NullHub")] {
            let output = std::process::Command::new(
                std::env::var("D2RHUB_MOD_PROCESSOR")
                    .expect("Set D2RHUB_MOD_PROCESSOR to the independent processor EXE"),
            )
            .args([
                "lightweight",
                "--game",
                &game,
                "--profile",
                profile,
                "--name",
                name,
                "--output",
            ])
            .arg(&parent)
            .arg("--events")
            .output()
            .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                .find(|v| v["type"] == "completed")
                .expect("completed event")["report"]
                .clone();
            let request = GenerateRequest {
                edition: "Global".into(),
                profile: profile.into(),
                mod_name: name.into(),
            };
            validate_output(&parent.join(name), &request, &report).unwrap();
            println!(
                "{name}: {} files, {} bytes",
                report["counts"]["verified_files"], report["generated_bytes"]
            );
        }
        fs::remove_dir_all(parent).unwrap();
    }
    #[test]
    fn all_profiles_default_to_txt_and_asset_test_mode() {
        assert_eq!(
            arguments("main", "LiteHub").unwrap(),
            "-mod LiteHub -txt -assettestmode 1"
        );
        assert_eq!(
            arguments("filler", "BoHub").unwrap(),
            "-mod BoHub -txt -assettestmode 1"
        );
        assert_eq!(
            arguments("min", "NullHub").unwrap(),
            "-mod NullHub -txt -assettestmode 1"
        );
        for name in ["../bad", "NUL", "con", "name with space", ""] {
            assert!(arguments("min", name).is_err());
        }
    }
    #[test]
    fn rejects_mismatched_identity_and_flags() {
        let report = serde_json::json!({"producer":PRODUCER,"mode":"bundled_rebuild","mod_name":"Wrong","profile":"min","launch_arguments":"-mod Wrong -txt","verified_output_integrity":true});
        assert!(metadata(&report, Path::new("unused"), "NullHub").is_err());
    }
    fn fixture() -> (PathBuf, Value, GenerateRequest) {
        let root =
            std::env::temp_dir().join(format!("hub-lightweight-test-{}", uuid::Uuid::new_v4()));
        let mpq = root.join("NullHub.mpq");
        fs::create_dir_all(mpq.join("data/global")).unwrap();
        fs::write(
            mpq.join("modinfo.json"),
            br#"{"name":"NullHub","savepath":"../"}"#,
        )
        .unwrap();
        fs::write(mpq.join("data/global/dataversionbuild.txt"), b"93854").unwrap();
        let (files, bytes) = tree_totals(&mpq).unwrap();
        let report = serde_json::json!({"producer":PRODUCER,"mode":"bundled_rebuild","profile":"min","mod_name":"NullHub","mod_directory":root.canonicalize().unwrap(),"launch_arguments":"-mod NullHub","game_data_version":"93854","verified_output_integrity":true,"counts":{"verified_files":files},"generated_bytes":bytes});
        fs::write(root.join(MANIFEST), serde_json::to_vec(&report).unwrap()).unwrap();
        (
            root,
            report,
            GenerateRequest {
                edition: "Global".into(),
                profile: "min".into(),
                mod_name: "NullHub".into(),
            },
        )
    }
    #[test]
    fn validates_disk_output_and_rejects_missing_files_and_wrong_directory() {
        let (root, mut report, request) = fixture();
        assert_eq!(
            inspect(&root, "NullHub").unwrap().unwrap().arguments,
            "-mod NullHub -txt -assettestmode 1"
        );
        assert!(validate_output(&root, &request, &report).is_ok());
        report["launch_arguments"] = Value::String("-mod NullHub -assettestmode 1".into());
        fs::write(root.join(MANIFEST), serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(validate_output(&root, &request, &report).is_ok());
        report["launch_arguments"] = Value::String(arguments("min", "NullHub").unwrap());
        fs::write(root.join(MANIFEST), serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(validate_output(&root, &request, &report).is_ok());
        let mut wrong_flags = report.clone();
        wrong_flags["launch_arguments"] =
            Value::String("-mod NullHub -txt -assettestmode 0".into());
        assert!(metadata(&wrong_flags, &root, "NullHub").is_err());
        report["mod_directory"] =
            Value::String(std::env::temp_dir().to_string_lossy().into_owned());
        fs::write(root.join(MANIFEST), serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(validate_output(&root, &request, &report)
            .unwrap_err()
            .contains("之外"));
        fs::remove_file(root.join("NullHub.mpq/data/global/dataversionbuild.txt")).unwrap();
        assert!(inspect(&root, "NullHub").is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn names_alone_are_not_profile_identity_and_cleanup_stays_owned() {
        let (root, _, _) = fixture();
        fs::remove_file(root.join(MANIFEST)).unwrap();
        assert!(inspect(&root, "NullHub").unwrap().is_none());
        assert!(cleanup(&root, &std::env::temp_dir().canonicalize().unwrap()).is_err());
        assert!(root.exists());
        assert!(name_exists(
            root.parent().unwrap(),
            &root
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_ascii_uppercase()
        )
        .unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
