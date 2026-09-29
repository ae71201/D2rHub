//! Hub adapter for the processor's MPQ CLI. Conversion and recovery live there.
use crate::{
    application::task_runtime::TaskRequest, infrastructure::managed_process,
    mod_catalog::ModCapsulePool, state::SharedState,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tauri::Emitter;

pub(crate) fn has_pending_conversion(mod_directory: &Path) -> bool {
    mod_directory.join(".d2rhub-unpack.json").exists()
}

pub(crate) fn ensure_no_pending_conversion(mods: &Path, arguments: &str) -> Result<(), String> {
    if let Some(name) = crate::domain::mod_arguments::active_mod_name(arguments)? {
        if has_pending_conversion(&mods.join(&name)) {
            return Err(format!(
                "Mod“{name}”的解压尚未完成，请先在 Mod 库点击解压恢复"
            ));
        }
    }
    Ok(())
}

async fn require_capabilities(processor: &Path) -> Result<(), String> {
    let mut command = managed_process::command(processor);
    command.args(["capabilities", "--json"]);
    let mut supported = false;
    let output = managed_process::run(
        &mut command,
        Some(Duration::from_secs(5)),
        || false,
        |line| {
            if let Ok(value) = serde_json::from_slice::<Value>(line) {
                let capabilities = value["capabilities"].as_array();
                supported = value["schema_version"] == 1
                    && ["mpq_unpack_v1", "mpq_recover_v1"].iter().all(|name| {
                        capabilities
                            .is_some_and(|list| list.iter().any(|v| v.as_str() == Some(name)))
                    });
            }
            Ok(())
        },
    )
    .await?;
    if output.exit_code == Some(0) && supported {
        Ok(())
    } else {
        Err("当前加工器不支持 MPQ 解压，请在“下载与更新”中更新加工器后重试".into())
    }
}

async fn resolve_processor(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let processor = crate::mod_resources::resolve_processor(app).await?;
    require_capabilities(&processor).await?;
    Ok(processor)
}

async fn invoke_cli(
    processor: &Path,
    root: &Path,
    name: &str,
    recovery: bool,
    cancelled: &(dyn Fn() -> bool + Sync),
    progress: &mut (dyn FnMut(&str, u8) + Send),
) -> Result<Value, String> {
    let mut command = crate::processor_pairing::command(processor);
    let operation = if recovery {
        "recover_mpq"
    } else {
        "unpack_mpq"
    };
    if recovery {
        command.arg("recover-mpq").arg("--mod-directory").arg(root);
    } else {
        command
            .arg("unpack-mpq")
            .arg("--source")
            .arg(root.join(format!("{name}.mpq")));
    }
    command.arg("--events");
    let mut report = None;
    let mut error = None;
    let output = managed_process::run(
        &mut command,
        Some(Duration::from_secs(30 * 60)),
        cancelled,
        |line| {
            let value: Value =
                serde_json::from_slice(line).map_err(|e| format!("加工器解压输出格式错误：{e}"))?;
            if value["operation"].as_str() != Some(operation) {
                return Err("加工器返回了不匹配的操作结果".into());
            }
            match value["type"].as_str() {
                Some("progress") => progress(
                    value["phase"].as_str().unwrap_or("working"),
                    value["percent"].as_u64().unwrap_or(0).min(99) as u8,
                ),
                Some("completed") => {
                    if report.is_some() {
                        return Err("加工器重复返回完成结果".into());
                    }
                    report = Some(value["report"].clone());
                }
                Some("error") => {
                    error = Some(managed_process::bounded_diagnostic(
                        value["message"].as_str().unwrap_or("MPQ 解压失败"),
                    ))
                }
                _ => return Err("无法识别加工器解压事件".into()),
            }
            Ok(())
        },
    )
    .await?;
    if output.exit_code != Some(0) || error.is_some() {
        return Err(error.unwrap_or_else(|| {
            if output.stderr.is_empty() {
                format!("加工器退出码：{:?}", output.exit_code)
            } else {
                output.stderr
            }
        }));
    }
    let report = report.ok_or("加工器没有返回解压完成结果")?;
    if report["schema_version"] != 1 {
        return Err("加工器解压报告版本不兼容".into());
    }
    Ok(report)
}

fn validate_report(root: &Path, name: &str, report: &Value) -> Result<(), String> {
    if !matches!(
        report["status"].as_str(),
        Some("converted" | "committed" | "already_directory")
    ) || report["mod_name"].as_str() != Some(name)
    {
        return Err("加工器返回的 Mod 名称或转换状态不匹配".into());
    }
    let canonical =
        |p: &Path| std::fs::canonicalize(p).map_err(|e| format!("验证解压目录失败：{e}"));
    let expected = canonical(root)?;
    let expected_mpq = root.join(format!("{name}.mpq"));
    if canonical(&expected_mpq)?.parent() != Some(expected.as_path()) {
        return Err("解压目录越过了当前 Mod 边界".into());
    }
    if has_pending_conversion(root)
        || !expected_mpq.is_dir()
        || canonical(Path::new(
            report["mod_directory"].as_str().ok_or("缺少 Mod 目录")?,
        ))? != expected
        || canonical(Path::new(
            report["mpq_directory"].as_str().ok_or("缺少 MPQ 目录")?,
        ))? != canonical(&expected_mpq)?
    {
        return Err("加工器返回的解压路径或磁盘状态不匹配".into());
    }
    if report["status"] != "already_directory" {
        let backup = Path::new(
            report["backup_path"]
                .as_str()
                .ok_or("加工器未返回备份位置")?,
        );
        if !backup.is_file() || !canonical(backup)?.starts_with(expected.join("back")) {
            return Err("加工器返回的 MPQ 备份路径无效".into());
        }
    }
    Ok(())
}

async fn convert(
    processor: &Path,
    root: &Path,
    name: &str,
    cancelled: &(dyn Fn() -> bool + Sync),
    progress: &mut (dyn FnMut(&str, u8) + Send),
) -> Result<Value, String> {
    let result = invoke_cli(processor, root, name, false, cancelled, progress)
        .await
        .and_then(|report| {
            validate_report(root, name, &report)?;
            Ok(report)
        });
    if let Err(reason) = result {
        if has_pending_conversion(root) {
            progress("recover", 99);
            // managed_process reaps the old child first. Recovery must not inherit cancellation.
            invoke_cli(processor, root, name, true, &|| false, &mut |_, _| {})
                .await
                .map_err(|e| format!("{reason}；加工器恢复失败：{e}。请再次点击解压恢复"))?;
            if has_pending_conversion(root) {
                return Err(format!("{reason}；解压事务仍待恢复，请再次点击解压"));
            }
        }
        return Err(reason);
    }
    result
}

#[derive(Clone, Serialize)]
struct Progress {
    capsule_id: String,
    task_id: u64,
    percent: u8,
    message: String,
}
#[derive(Serialize)]
pub struct UnpackResult {
    pool: ModCapsulePool,
    backup_path: Option<String>,
    escaped_name_count: usize,
}

#[tauri::command]
pub async fn unpack_mod_capsule(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    capsule_id: String,
) -> Result<UnpackResult, String> {
    let shared = state.inner().clone();
    let _mutation = shared.mod_mutations().try_acquire()?;
    let (config, root, name) = crate::mod_catalog::resolve_unpack_target(&shared, &capsule_id)?;
    crate::audio_mod::ensure_audio_mod_not_in_use(&shared, &config, &name)?;
    let task = shared
        .tasks()
        .begin(
            TaskRequest::new("mod-mpq-unpack")
                .for_subject(&capsule_id)
                .with_conflict_key("audio-mod-build")
                .non_retryable()
                .with_initial_status("preflight", "正在检查 MPQ 解压环境"),
        )
        .map_err(|e| e.to_string())?;
    let mut last_percent = 0;
    let mut progress = |phase: &str, percent: u8| {
        last_percent = last_percent.max(percent);
        let message = match phase {
            "preflight" => "正在检查 MPQ 解压环境",
            "inspect" => "正在检查 MPQ",
            "extract" => "正在解压 MPQ",
            "verify" => "正在校验解压资源",
            "backup" => "正在备份原 MPQ",
            "recover" => "正在恢复 MPQ 转换，请稍候",
            "complete" => "解压完成，正在刷新预设",
            _ => "正在处理 MPQ",
        };
        let _ = task.update(last_percent, phase, message);
        let _ = app.emit(
            "mod-mpq-progress",
            Progress {
                capsule_id: capsule_id.clone(),
                task_id: task.task_id(),
                percent: last_percent,
                message: message.into(),
            },
        );
    };
    progress("preflight", 0);
    let result = async {
        let processor = resolve_processor(&app).await?;
        convert(
            &processor,
            &root,
            &name,
            &|| task.cancellation_requested(),
            &mut progress,
        )
        .await
    }
    .await;
    match result {
        Ok(report) => {
            let pool = match crate::mod_catalog::get_mod_capsule_pool(app, state).await {
                Ok(pool) => pool,
                Err(e) => {
                    let _ = task.fail("mpq-refresh-failed", &e);
                    return Err(e);
                }
            };
            let _ = task.succeed("MPQ 已解压，可继续加工");
            Ok(UnpackResult {
                pool,
                backup_path: report["backup_path"].as_str().map(str::to_owned),
                escaped_name_count: report["escaped_file_names"].as_array().map_or(0, Vec::len),
            })
        }
        Err(error) => {
            if task.cancellation_requested() && !has_pending_conversion(&root) {
                let _ = task.cancelled(&error);
            } else {
                let _ = task.fail("mpq-unpack-failed", &error);
            }
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("hub-mpq-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn scans_mpq_as_unpack_only_and_preserves_identity_after_conversion() {
        let scratch = Scratch::new();
        let root = scratch.0.join("mini");
        std::fs::create_dir(&root).unwrap();
        let mpq = root.join("mini.mpq");
        std::fs::write(&mpq, b"MPQ archive fixture").unwrap();
        let before = crate::audio_mod::installed_mods(&scratch.0);
        assert_eq!(before.len(), 1);
        assert!(before[0].requires_unpack);
        assert!(!before[0].source_eligible);
        // A backup must not appear as a second preset.
        std::fs::create_dir_all(root.join("back/id")).unwrap();
        std::fs::rename(&mpq, root.join("back/id/mini.mpq")).unwrap();
        std::fs::create_dir(&mpq).unwrap();
        let after = crate::audio_mod::installed_mods(&scratch.0);
        assert_eq!(after.len(), 1);
        assert_eq!(before[0].name, after[0].name);
        assert!(!after[0].requires_unpack);
        assert!(after[0].source_eligible);
    }
    #[test]
    fn interrupted_archive_remains_visible_and_cannot_launch() {
        let scratch = Scratch::new();
        let root = scratch.0.join("mini");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join(".d2rhub-unpack.json"), "{}").unwrap();
        let mods = crate::audio_mod::installed_mods(&scratch.0);
        assert_eq!(mods.len(), 1);
        assert!(mods[0].requires_unpack && mods[0].unpack_recovery_required);
        assert!(ensure_no_pending_conversion(&scratch.0, "-mod mini -txt").is_err());
        assert!(ensure_no_pending_conversion(&scratch.0, "-mod Other -txt").is_ok());
    }
    #[test]
    fn refuses_mismatched_cli_identity_paths_and_pending_commit() {
        let scratch = Scratch::new();
        let root = scratch.0.join("mini");
        std::fs::create_dir_all(root.join("mini.mpq")).unwrap();
        let mut report = json!({"status":"already_directory","mod_name":"mini","mod_directory":root,"mpq_directory":root.join("mini.mpq")});
        assert!(validate_report(&root, "mini", &report).is_ok());
        report["mod_name"] = json!("Other");
        assert!(validate_report(&root, "mini", &report).is_err());
        report["mod_name"] = json!("mini");
        report["mpq_directory"] = json!(scratch.0);
        assert!(validate_report(&root, "mini", &report).is_err());
        report["mpq_directory"] = json!(root.join("mini.mpq"));
        std::fs::write(root.join(".d2rhub-unpack.json"), "{}").unwrap();
        assert!(validate_report(&root, "mini", &report).is_err());
    }
    #[tokio::test]
    #[ignore = "requires D2RHUB_TEST_MOD_PROCESSOR and D2RHUB_TEST_MPQ"]
    async fn real_processor_unpacks_original_mpq_and_refreshes_scanned_source() {
        let processor = PathBuf::from(std::env::var_os("D2RHUB_TEST_MOD_PROCESSOR").unwrap());
        let source = PathBuf::from(std::env::var_os("D2RHUB_TEST_MPQ").unwrap());
        require_capabilities(&processor).await.unwrap();
        let scratch = Scratch::new();
        let name = source.file_stem().unwrap().to_str().unwrap();
        let root = scratch.0.join(name);
        std::fs::create_dir(&root).unwrap();
        let mpq = root.join(format!("{name}.mpq"));
        std::fs::copy(&source, &mpq).unwrap();
        assert!(crate::audio_mod::installed_mods(&scratch.0)[0].requires_unpack);
        let mut progress = Vec::new();
        let report = convert(&processor, &root, name, &|| false, &mut |phase, percent| {
            progress.push((phase.to_string(), percent))
        })
        .await
        .unwrap();
        assert_eq!(report["status"], "converted");
        assert!(progress.iter().any(|(phase, _)| phase == "extract"));
        let after = crate::audio_mod::installed_mods(&scratch.0);
        assert!(!after[0].requires_unpack);
        assert!(after[0].source_eligible);
        assert_eq!(
            std::fs::read(report["backup_path"].as_str().unwrap()).unwrap(),
            std::fs::read(source).unwrap()
        );
    }
}
