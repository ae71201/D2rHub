//! Account-free installation and running-session resolution. Never registers an account
//! or writes launcher/account settings. Only verified process metadata reaches capture.
use crate::domain::config::{ExternalAudioTarget, GlobalConfig};
use crate::domain::mod_arguments::{active_mod_name, has_txt_argument};
use crate::state::SharedState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub(crate) fn installation_directory(
    config: &GlobalConfig,
    edition: &str,
) -> Result<PathBuf, String> {
    let path = match edition {
        "CN" => &config.cn_game_path,
        "Global" => &config.global_game_path,
        _ => return Err("请选择国服或国际服游戏目录".into()),
    };
    let directory = PathBuf::from(path.trim());
    if path.trim().is_empty() || !directory.join("D2R.exe").is_file() {
        return Err("请先在运行环境中配置有效的游戏安装目录（包含 D2R.exe）".into());
    }
    Ok(directory)
}

pub(crate) fn source_id(config: &GlobalConfig, edition: &str) -> String {
    let path = if edition == "Global" {
        &config.global_game_path
    } else {
        &config.cn_game_path
    };
    format!(
        "external:{edition}:{}",
        normalized_path(Path::new(path.trim()))
    )
}

pub(crate) fn source_name(config: &GlobalConfig, edition: &str) -> String {
    match (config.app_language.as_str(), edition) {
        ("en-US", "Global") => "Local game · Global",
        ("en-US", _) => "Local game · China",
        (_, "Global") => "本机游戏 · 国际服",
        _ => "本机游戏 · 国服",
    }
    .into()
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_start_matches("\\\\?\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

pub(crate) fn validate_target(
    config: &GlobalConfig,
    target: &ExternalAudioTarget,
) -> Result<(), String> {
    installation_directory(config, &target.edition)?;
    crate::domain::mod_arguments::plain_mod_name(&target.mod_name)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub started_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAudioInstance {
    #[serde(flatten)]
    pub identity: ProcessIdentity,
    pub mod_name: Option<String>,
    pub ready: bool,
    pub selected: bool,
    pub window_title: Option<String>,
    pub message: String,
}

#[derive(Deserialize)]
struct ProcessMetadata {
    pid: u32,
    started_at: u64,
    executable: Option<String>,
    #[serde(default)]
    window_title: Option<String>,
    // May include launcher credentials. Never log, persist or serialize this value.
    command_line: Option<String>,
}

#[derive(Debug, Clone)]
struct GameProcess {
    identity: ProcessIdentity,
    executable: Option<PathBuf>,
    window_title: Option<String>,
    mod_name: Option<String>,
    has_txt: bool,
    metadata_verified: bool,
}

fn game_process(metadata: ProcessMetadata) -> GameProcess {
    let arguments = metadata
        .command_line
        .as_deref()
        .filter(|arguments| !arguments.trim().is_empty());
    let parsed_mod = arguments.map(active_mod_name).transpose();
    let parsed_txt = arguments.map(has_txt_argument).transpose();
    let metadata_verified = arguments.is_some() && parsed_mod.is_ok() && parsed_txt.is_ok();
    GameProcess {
        identity: ProcessIdentity {
            pid: metadata.pid,
            started_at: metadata.started_at,
        },
        executable: metadata
            .executable
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
        window_title: metadata
            .window_title
            .filter(|title| !title.trim().is_empty()),
        mod_name: parsed_mod.ok().flatten().flatten(),
        has_txt: parsed_txt.ok().flatten().unwrap_or(false),
        metadata_verified,
    }
}

#[cfg(target_os = "windows")]
fn scan_processes() -> Result<Vec<GameProcess>, String> {
    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        sysinfo::ProcessRefreshKind::new(),
    );
    if !system
        .processes()
        .values()
        .any(|process| process.name().eq_ignore_ascii_case("D2R.exe"))
    {
        return Ok(Vec::new());
    }
    // Static command: no user paths/arguments enter PowerShell source. WMI reads
    // operating-system process metadata; no ReadProcessMemory or game injection.
    const SCRIPT: &str = r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new(); $rows=@(Get-CimInstance Win32_Process -Filter "Name='D2R.exe'" | ForEach-Object { @{pid=[uint32]$_.ProcessId; started_at=([DateTimeOffset]$_.CreationDate).ToUnixTimeSeconds(); executable=$_.ExecutablePath; window_title=(Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue).MainWindowTitle; command_line=$_.CommandLine} }); ConvertTo-Json -InputObject $rows -Compress"#;
    let mut command = crate::infrastructure::process::silent_cmd("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT]);
    let output = read_process_output(&mut command, Duration::from_secs(5))?;
    let bytes = output.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&output);
    let metadata: Vec<ProcessMetadata> =
        serde_json::from_slice(bytes).map_err(|_| "系统返回的游戏进程信息无效".to_string())?;
    Ok(metadata.into_iter().map(game_process).collect())
}

#[cfg(target_os = "windows")]
fn read_process_output(
    command: &mut std::process::Command,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    use std::io::Read;
    use std::process::Stdio;

    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "无法读取游戏进程信息，请检查系统 PowerShell / WMI 服务".to_string())?;
    let mut stdout = child
        .stdout
        .take()
        .expect("process metadata stdout is piped");
    // Drain while the child runs: a full pipe must not prevent its exit.
    // The scope also joins the reader after timeout termination and reaping.
    std::thread::scope(|scope| {
        let reader = scope.spawn(move || {
            let mut output = Vec::new();
            stdout.read_to_end(&mut output).map(|_| output)
        });
        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("读取游戏进程信息超时，请稍后重试".into());
                }
            }
        };
        if !status.success() {
            return Err("无法读取游戏进程信息，请检查系统 WMI 服务与运行权限".into());
        }
        reader
            .join()
            .map_err(|_| "读取游戏进程信息失败".to_string())?
            .map_err(|_| "读取游戏进程信息失败".to_string())
    })
}

#[cfg(not(target_os = "windows"))]
fn scan_processes() -> Result<Vec<GameProcess>, String> {
    Err("指定游戏进程目前只支持 Windows".into())
}

type ProcessCache = Option<(Instant, Result<Vec<GameProcess>, String>)>;
fn processes() -> Result<Vec<GameProcess>, String> {
    static CACHE: OnceLock<Mutex<ProcessCache>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some((at, result)) = cache.as_ref() {
        if at.elapsed() < Duration::from_secs(2) {
            return result.clone();
        }
    }
    let result = scan_processes();
    *cache = Some((Instant::now(), result.clone()));
    result
}

fn matches_installation(process: &GameProcess, directory: &Path) -> bool {
    process
        .executable
        .as_deref()
        .is_some_and(|path| normalized_path(path) == normalized_path(&directory.join("D2R.exe")))
}

fn inspect_instance(process: &GameProcess, target: &ExternalAudioTarget) -> ExternalAudioInstance {
    let message = if !process.metadata_verified {
        "无法确认游戏启动参数，请以与游戏相同的权限运行 D2RHub".into()
    } else if !process.has_txt
        || !process
            .mod_name
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case(&target.mod_name))
    {
        format!(
            "游戏尚未使用所选识别 Mod，请配置 -mod {} -txt 并重启游戏",
            target.mod_name
        )
    } else {
        String::new()
    };
    ExternalAudioInstance {
        identity: process.identity.clone(),
        mod_name: process.mod_name.clone(),
        ready: message.is_empty(),
        selected: false,
        window_title: process.window_title.clone(),
        message,
    }
}

#[derive(Default)]
struct Binding {
    key: String,
    selected: Option<ProcessIdentity>,
}
fn binding() -> &'static Mutex<Binding> {
    static BINDING: OnceLock<Mutex<Binding>> = OnceLock::new();
    BINDING.get_or_init(|| Mutex::new(Binding::default()))
}
fn binding_key(config: &GlobalConfig, target: &ExternalAudioTarget) -> String {
    format!(
        "{}:{}",
        source_id(config, &target.edition),
        target.mod_name.to_lowercase()
    )
}

fn choose_instance(
    binding: &mut Binding,
    key: &str,
    instances: &[ExternalAudioInstance],
) -> Result<ProcessIdentity, String> {
    if binding.key != key {
        *binding = Binding {
            key: key.into(),
            ..Default::default()
        };
    }
    if let Some(selected) = binding.selected.as_ref() {
        if let Some(instance) = instances
            .iter()
            .find(|instance| &instance.identity == selected)
        {
            return if instance.ready {
                Ok(selected.clone())
            } else {
                Err(instance.message.clone())
            };
        }
        binding.selected = None;
        return Err("所选游戏已退出，请重新指定游戏进程".into());
    }
    match instances {
        [] => Err("等待游戏运行；启动后请选择要监听的游戏进程".into()),
        [instance] if !instance.ready => Err(instance.message.clone()),
        _ => Err("请选择要监听的游戏进程".into()),
    }
}
pub(crate) fn instances(
    config: &GlobalConfig,
    target: &ExternalAudioTarget,
) -> Result<Vec<ExternalAudioInstance>, String> {
    let directory = installation_directory(config, &target.edition)?;
    let scanned = processes()?;
    let mut result: Vec<_> = scanned
        .iter()
        .filter(|process| matches_installation(process, &directory))
        .map(|process| inspect_instance(process, target))
        .collect();
    if result.is_empty() && scanned.iter().any(|process| process.executable.is_none()) {
        return Err("检测到游戏，但无法确认安装路径，请以与游戏相同的权限运行 D2RHub".into());
    }
    result.sort_by_key(|instance| instance.identity.pid);
    Ok(result)
}

pub(crate) fn resolve_session(
    config: &GlobalConfig,
    target: &ExternalAudioTarget,
) -> Result<ProcessIdentity, String> {
    let instances = instances(config, target)?;
    let identity = choose_instance(
        &mut binding().lock().unwrap_or_else(|e| e.into_inner()),
        &binding_key(config, target),
        &instances,
    )?;
    verify_process_identity(&identity)?;
    Ok(identity)
}

fn verify_process_identity(identity: &ProcessIdentity) -> Result<(), String> {
    let pid = sysinfo::Pid::from_u32(identity.pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[pid]),
        sysinfo::ProcessRefreshKind::new(),
    );
    if system.process(pid).is_some_and(|process| {
        process.start_time() == identity.started_at
            && process.name().eq_ignore_ascii_case("D2R.exe")
    }) {
        Ok(())
    } else {
        Err("等待游戏重新运行；上一个游戏实例已退出".into())
    }
}

pub(crate) fn current_session_matches(
    config: &GlobalConfig,
    target: &ExternalAudioTarget,
    pid: Option<u32>,
) -> bool {
    let selected = {
        let binding = binding().lock().unwrap_or_else(|e| e.into_inner());
        if binding.key != binding_key(config, target) {
            return false;
        }
        binding.selected.clone()
    };
    let Some(identity) = selected.filter(|identity| pid == Some(identity.pid)) else {
        return false;
    };
    verify_process_identity(&identity).is_ok()
}

pub(crate) fn ensure_mod_not_in_use(directory: &Path, mod_name: &str) -> Result<(), String> {
    for process in processes()? {
        if blocks_mod_update(&process, directory, mod_name) {
            return Err(format!(
                "请先关闭正在使用或无法确认 Mod 的游戏（PID {}），再更新 Mod“{mod_name}”",
                process.identity.pid
            ));
        }
    }
    Ok(())
}

fn blocks_mod_update(process: &GameProcess, directory: &Path, mod_name: &str) -> bool {
    process.executable.is_none()
        || (matches_installation(process, directory)
            && (!process.metadata_verified
                || process
                    .mod_name
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(mod_name))))
}

#[tauri::command]
pub async fn get_external_audio_instances(
    state: tauri::State<'_, SharedState>,
) -> Result<Vec<ExternalAudioInstance>, String> {
    let config = state.configuration().snapshot().ok_or("识别设置尚未加载")?;
    let target = config
        .rune_audio_external_target
        .clone()
        .ok_or("尚未选择指定游戏进程模式")?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut instances = instances(&config, &target)?;
        let binding = binding().lock().unwrap_or_else(|e| e.into_inner());
        if binding.key == binding_key(&config, &target) {
            for instance in &mut instances {
                instance.selected = binding.selected.as_ref() == Some(&instance.identity);
            }
        }
        Ok(instances)
    })
    .await
    .map_err(|_| "读取游戏实例失败".to_string())?
}

#[tauri::command]
pub async fn select_external_audio_instance(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    identity: ProcessIdentity,
) -> Result<(), String> {
    let config = state.configuration().snapshot().ok_or("识别设置尚未加载")?;
    let target = config
        .rune_audio_external_target
        .clone()
        .ok_or("尚未选择指定游戏进程模式")?;
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<SharedState>();
        let _activation = state
            .runtime_activation_lock
            .try_lock()
            .ok_or("模式切换或模块操作进行中，请稍后重试")?;
        let instances = instances(&config, &target)?;
        let found = instances
            .iter()
            .find(|instance| instance.identity == identity)
            .ok_or("所选游戏已退出，请重新选择")?;
        if !found.ready {
            return Err(found.message.clone());
        }
        verify_process_identity(&identity)?;
        super::monitor::stop_blocking()?;
        let selected = Binding {
            key: binding_key(&config, &target),
            selected: Some(identity),
        };
        *binding().lock().unwrap_or_else(|e| e.into_inner()) = selected;
        if config.rune_audio_enabled {
            super::monitor::start_capability(app.clone())?;
        }
        Ok(())
    })
    .await
    .map_err(|_| "连接游戏实例失败".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::slice;

    #[cfg(target_os = "windows")]
    #[test]
    fn process_scan_drains_output_larger_than_the_pipe_capacity() {
        let mut command = crate::infrastructure::process::silent_cmd("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::Out.Write('x' * 262144)",
        ]);
        let output = read_process_output(&mut command, Duration::from_secs(5)).unwrap();
        assert_eq!(output, vec![b'x'; 262144]);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn process_scan_timeout_terminates_child_and_joins_output_reader() {
        let mut command = crate::infrastructure::process::silent_cmd("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::Out.Write('x' * 262144); Start-Sleep -Seconds 30",
        ]);
        let started = Instant::now();
        let error = read_process_output(&mut command, Duration::from_secs(2)).unwrap_err();
        assert!(error.contains("超时"));
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    fn instance(pid: u32, started_at: u64) -> ExternalAudioInstance {
        ExternalAudioInstance {
            identity: ProcessIdentity { pid, started_at },
            mod_name: Some("test".into()),
            ready: true,
            selected: false,
            window_title: None,
            message: String::new(),
        }
    }
    #[test]
    fn every_game_requires_explicit_selection_including_a_single_instance() {
        for instances in [
            vec![],
            vec![instance(1, 10)],
            vec![instance(1, 10), instance(2, 20)],
        ] {
            let mut binding = Binding::default();
            assert!(choose_instance(&mut binding, "local", &instances).is_err());
            assert!(binding.selected.is_none());
        }
    }
    #[test]
    fn selected_game_is_sticky_when_peers_arrive_or_exit() {
        let first = instance(1, 10);
        let peer = instance(2, 20);
        let mut binding = Binding {
            key: "local".into(),
            selected: Some(first.identity.clone()),
        };
        for peers in [vec![first.clone()], vec![first.clone(), peer]] {
            assert_eq!(
                choose_instance(&mut binding, "local", &peers).unwrap(),
                first.identity
            );
        }
    }
    #[test]
    fn exited_or_reused_pid_requires_a_new_manual_selection() {
        for replacement in [instance(2, 20), instance(1, 30)] {
            let mut binding = Binding {
                key: "local".into(),
                selected: Some(instance(1, 10).identity),
            };
            assert!(choose_instance(&mut binding, "local", slice::from_ref(&replacement)).is_err());
            assert!(binding.selected.is_none());
            assert!(choose_instance(&mut binding, "local", slice::from_ref(&replacement)).is_err());
        }
    }
    #[test]
    fn changed_installation_or_mod_clears_process_selection() {
        let first = instance(1, 10);
        let mut binding = Binding {
            key: "local".into(),
            selected: Some(first.identity.clone()),
        };
        assert!(choose_instance(&mut binding, "other", &[first]).is_err());
        assert!(binding.selected.is_none());
        assert_eq!(binding.key, "other");
    }
    #[test]
    fn selected_process_with_invalid_arguments_cannot_be_captured() {
        let mut game = instance(1, 10);
        game.ready = false;
        game.message = "restart required".into();
        let mut binding = Binding {
            key: "local".into(),
            selected: Some(game.identity.clone()),
        };
        assert_eq!(
            choose_instance(&mut binding, "local", &[game]).unwrap_err(),
            "restart required"
        );
    }
    #[test]
    fn process_metadata_checks_actual_arguments_and_discards_credentials() {
        let process = game_process(ProcessMetadata {
            pid: 42,
            started_at: 1,
            executable: Some("C:\\Game\\D2R.exe".into()),
            window_title: Some("Main account".into()),
            command_line: Some(r#""C:\Game\D2R.exe" -username secret -mod "my mod" -txt"#.into()),
        });
        let target = ExternalAudioTarget {
            edition: "CN".into(),
            mod_name: "my mod".into(),
        };
        assert!(inspect_instance(&process, &target).ready);
        assert!(!format!("{process:?}").contains("secret"));
        let unreadable = game_process(ProcessMetadata {
            pid: 42,
            started_at: 1,
            executable: None,
            window_title: None,
            command_line: None,
        });
        assert!(!inspect_instance(&unreadable, &target).ready);
        assert!(blocks_mod_update(
            &unreadable,
            Path::new("C:\\Game"),
            "my mod"
        ));
        assert!(blocks_mod_update(&process, Path::new("C:\\Game"), "my mod"));
        assert!(!blocks_mod_update(
            &process,
            Path::new("C:\\Other"),
            "my mod"
        ));
    }
}
