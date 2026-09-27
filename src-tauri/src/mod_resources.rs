//! Optional, versioned Release assets. No executable or Mod is bundled in Hub.
use crate::{
    application::task_runtime::{TaskHandle, TaskRequest},
    audio_mod::BuildLease,
    infrastructure::durable_fs,
    state::SharedState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

const CHANNEL: &str = "v7-r25-r28-lightweight-v1";
const INDEX_URL: &str =
    "https://raw.githubusercontent.com/gjy991229/D2rHub/main/resources/mod-resources-v1.json";
const EMBEDDED: &str = include_str!("../../resources/mod-resources-v1.json");
static CATALOG: Mutex<Option<Catalog>> = Mutex::new(None);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    id: String,
    version: String,
    url: String,
    size: u64,
    sha256: String,
    mod_name: Option<String>,
    profile: Option<String>,
    game_data_version: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    schema: u32,
    channel: String,
    revision: u64,
    release_url: String,
    assets: Vec<Asset>,
}
#[derive(Serialize)]
pub struct ProcessorStatus {
    pub ready: bool,
    pub installed_version: Option<String>,
    pub recommended_version: String,
    pub installed_path: Option<String>,
    pub install_directory: String,
    pub legacy: bool,
}
#[derive(Serialize)]
pub struct ResourceState {
    catalog: Catalog,
    processor: ProcessorStatus,
    mods_directory: Option<String>,
    game_data_version: Option<String>,
    warning: Option<String>,
}
#[derive(Serialize)]
pub struct InstallResult {
    path: String,
    task_id: u64,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn safe_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() < 100
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
        && s != "."
        && s != ".."
}
fn validate_catalog(c: &Catalog) -> Result<(), String> {
    if c.schema != 1
        || c.channel != CHANNEL
        || c.assets.len() != 4
        || !c
            .release_url
            .starts_with("https://github.com/gjy991229/D2rHub/releases/tag/mod-resources-")
    {
        return Err("资源清单与当前 Hub 不兼容".into());
    }
    let mut ids = HashSet::new();
    for a in &c.assets {
        let expected = match a.id.as_str() {
            "processor" => None,
            "LiteHub" => Some("main"),
            "BoHub" => Some("filler"),
            "NullHub" => Some("min"),
            _ => return Err("未知资源".into()),
        };
        let url = reqwest::Url::parse(&a.url).map_err(err)?;
        if !ids.insert(&a.id)
            || !safe_token(&a.version)
            || a.size == 0
            || a.size > 1024 * 1024 * 1024
            || a.sha256.len() != 64
            || !a.sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || url.scheme() != "https"
            || url.host_str() != Some("github.com")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !url
                .path()
                .starts_with("/gjy991229/D2rHub/releases/download/mod-resources-")
            || a.profile.as_deref() != expected
            || a.mod_name.as_deref() != expected.map(|_| a.id.as_str())
            || (expected.is_some()
                && !a
                    .game_data_version
                    .as_ref()
                    .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit())))
        {
            return Err("资源地址、校验值或身份无效".into());
        }
    }
    Ok(())
}
fn catalog() -> Result<Catalog, String> {
    if let Some(c) = CATALOG.lock().map_err(err)?.clone() {
        return Ok(c);
    }
    let c = serde_json::from_str(EMBEDDED).map_err(err)?;
    validate_catalog(&c)?;
    Ok(c)
}
fn processor_asset(c: &Catalog) -> &Asset {
    c.assets
        .iter()
        .find(|a| a.id == "processor")
        .expect("validated catalog")
}
fn tools_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(err)?
        .join("tools")
        .join("d2r-audio-mod"))
}
fn processor_path(root: &Path, a: &Asset) -> PathBuf {
    root.join(format!("{}-{}", a.version, &a.sha256[..12]))
        .join("d2r-audio-mod.exe")
}
fn digest(path: &Path) -> Result<String, String> {
    let mut input = fs::File::open(path).map_err(err)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = input.read(&mut buffer).map_err(err)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn verify_file(path: &Path, a: &Asset) -> Result<(), String> {
    if fs::metadata(path).map_err(err)?.len() != a.size || digest(path)? != a.sha256.to_lowercase()
    {
        return Err("文件大小或 SHA-256 校验失败，请重新下载".into());
    }
    Ok(())
}
pub(crate) fn resolve_processor(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let c = catalog()?;
    let a = processor_asset(&c);
    let path = processor_path(&tools_root(app)?, a);
    verify_file(&path, a)
        .map_err(|_| format!("请先在 Mod 资源下载中安装或更新加工器至 {}", a.version))?;
    Ok(path)
}
async fn probe_version(app: &tauri::AppHandle, path: &Path) -> Option<String> {
    // Spawn explicitly so timeout also terminates the child.
    use tauri_plugin_shell::process::CommandEvent;
    let (mut events, child) = app.shell().command(path).arg("--version").spawn().ok()?;
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        let mut version = None;
        while let Some(event) = events.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    let line = String::from_utf8_lossy(&bytes);
                    let mut words = line.split_whitespace();
                    if words.next() == Some("d2r-audio-mod") {
                        version = words.next().filter(|s| safe_token(s)).map(str::to_owned);
                    }
                }
                CommandEvent::Terminated(info) => {
                    return if info.code == Some(0) { version } else { None }
                }
                _ => {}
            }
        }
        None
    })
    .await;
    match result {
        Ok(v) => v,
        Err(_) => {
            let _ = child.kill();
            None
        }
    }
}
async fn processor_status(app: &tauri::AppHandle, c: &Catalog) -> Result<ProcessorStatus, String> {
    let root = tools_root(app)?;
    let a = processor_asset(c);
    let path = processor_path(&root, a);
    let ready = verify_file(&path, a).is_ok();
    let mut status = ProcessorStatus {
        ready,
        installed_version: ready.then(|| a.version.clone()),
        recommended_version: a.version.clone(),
        installed_path: ready.then(|| path.to_string_lossy().into_owned()),
        install_directory: root.to_string_lossy().into_owned(),
        legacy: false,
    };
    if ready {
        return Ok(status);
    }
    // Check the previous managed version and the historical bundled location.
    let mut candidates = Vec::new();
    if let Ok(bytes) = fs::read(root.join("installed.json")) {
        if let Ok(old) = serde_json::from_slice::<Catalog>(&bytes) {
            if validate_catalog(&old).is_ok() {
                let asset = processor_asset(&old);
                let path = processor_path(&root, asset);
                if verify_file(&path, asset).is_ok() {
                    candidates.push((path, false));
                }
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push((parent.join("d2r-audio-mod.exe"), true));
        }
    }
    if let Ok(dir) = app.path().resource_dir() {
        candidates.push((dir.join("d2r-audio-mod.exe"), true));
    }
    for (path, legacy) in candidates {
        if path.is_file() {
            status.installed_version = probe_version(app, &path).await;
            status.installed_path = Some(path.to_string_lossy().into_owned());
            status.legacy = legacy;
            break;
        }
    }
    Ok(status)
}
fn game_root(state: &SharedState, edition: &str) -> Result<PathBuf, String> {
    crate::lightweight_mod::game_path(state, edition)
}
fn game_version(root: &Path) -> Result<String, String> {
    let text = fs::read_to_string(root.join(".build.info")).map_err(err)?;
    parse_game_version(&text)
}
fn parse_game_version(text: &str) -> Result<String, String> {
    let mut lines = text.lines();
    let headers: Vec<_> = lines.next().ok_or("缺少游戏版本信息")?.split('|').collect();
    let active = headers
        .iter()
        .position(|h| h.starts_with("Active!"))
        .ok_or("缺少激活版本")?;
    let version = headers
        .iter()
        .position(|h| h.starts_with("Version!"))
        .ok_or("缺少游戏版本")?;
    for line in lines {
        let cells: Vec<_> = line.split('|').collect();
        if cells.get(active) == Some(&"1") {
            let value = cells
                .get(version)
                .and_then(|s| s.rsplit('.').next())
                .ok_or("游戏版本无效")?;
            if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
                return Ok(value.into());
            }
        }
    }
    Err("无法识别当前游戏数据版本".into())
}
fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("D2RHub-ModResources/1")
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(900))
        .build()
        .map_err(err)
}
async fn remote_catalog() -> Result<Catalog, String> {
    let mut response = client()?
        .get(INDEX_URL)
        .timeout(Duration::from_secs(12))
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(err)? {
        if bytes.len() + chunk.len() > 65536 {
            return Err("资源清单过大".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let c = serde_json::from_slice(&bytes).map_err(err)?;
    validate_catalog(&c)?;
    Ok(c)
}
#[tauri::command]
pub async fn get_mod_resources(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    edition: String,
    refresh: bool,
) -> Result<ResourceState, String> {
    let cache_path = tools_root(&app)?.join("catalog.json");
    let (mut c, _) = publish_catalog(&CATALOG, &cache_path, catalog()?, false)?;
    let mut warning = None;
    if refresh {
        match remote_catalog().await {
            Ok(remote) => {
                (c, warning) = publish_catalog(&CATALOG, &cache_path, remote, true)?;
            }
            Err(e) => warning = Some(format!("暂时无法检查更新，使用已知资源清单：{e}")),
        }
    }
    // Another refresh may have completed while this request was awaiting the network.
    (c, _) = publish_catalog(&CATALOG, &cache_path, c, false)?;
    let game = game_root(state.inner(), &edition);
    let (mods_directory, game_data_version) = match game {
        Ok(root) => (
            Some(root.join("mods").to_string_lossy().into_owned()),
            game_version(&root).ok(),
        ),
        Err(e) => {
            warning = Some(warning.map_or(e.clone(), |w| format!("{w}；{e}")));
            (None, None)
        }
    };
    Ok(ResourceState {
        processor: processor_status(&app, &c).await?,
        catalog: c,
        mods_directory,
        game_data_version,
        warning,
    })
}
fn publish_catalog(
    cache: &Mutex<Option<Catalog>>,
    path: &Path,
    mut candidate: Catalog,
    persist: bool,
) -> Result<(Catalog, Option<String>), String> {
    // Selection and durable publication share one lock. A late response must not
    // overwrite a newer disk cache even if it started from an older snapshot.
    let mut current = cache.lock().map_err(err)?;
    let requested_revision = candidate.revision;
    if let Ok(bytes) = fs::read(path) {
        if let Ok(saved) = serde_json::from_slice::<Catalog>(&bytes) {
            if validate_catalog(&saved).is_ok() && saved.revision >= candidate.revision {
                candidate = saved;
            }
        }
    }
    if let Some(newer) = current
        .as_ref()
        .filter(|c| c.revision >= candidate.revision)
    {
        candidate = newer.clone();
    }
    let mut warning = (persist && requested_revision < candidate.revision)
        .then(|| "远端资源清单较旧，继续使用已验证版本".to_string());
    if persist {
        let saved = (|| {
            fs::create_dir_all(path.parent().ok_or("资源缓存目录无效")?).map_err(err)?;
            save_catalog(path, &candidate)
        })();
        if let Err(e) = saved {
            let message = format!("已检查更新，但无法缓存：{e}");
            warning = Some(warning.map_or(message.clone(), |w| format!("{w}；{message}")));
        }
    }
    *current = Some(candidate.clone());
    Ok((candidate, warning))
}
fn save_catalog(path: &Path, c: &Catalog) -> Result<(), String> {
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(err)?;
        file.write_all(&serde_json::to_vec(c).map_err(err)?)
            .map_err(err)?;
        file.sync_all().map_err(err)?;
        drop(file);
        if path.exists() {
            durable_fs::durable_sibling_replace(&temp, path).map_err(err)
        } else {
            durable_fs::durable_sibling_rename(&temp, path).map_err(err)
        }
    })();
    let _ = fs::remove_file(&temp);
    result
}
fn cancelled(task: &TaskHandle) -> Result<(), String> {
    if task.cancellation_requested() {
        Err("下载或安装已取消".into())
    } else {
        Ok(())
    }
}
async fn await_cancellable<T>(
    task: &TaskHandle,
    operation: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    // Keep polling the same request; do not restart it at each cancellation check.
    tokio::pin!(operation);
    loop {
        cancelled(task)?;
        tokio::select! {
            result = &mut operation => {
                cancelled(task)?;
                return result;
            }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {}
        }
    }
}
async fn download(asset: &Asset, target: &Path, task: &TaskHandle) -> Result<(), String> {
    let mut response = await_cancellable(task, async {
        client()?.get(&asset.url).send().await.map_err(err)
    })
    .await?
    .error_for_status()
    .map_err(err)?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(target)
        .map_err(err)?;
    let mut size = 0;
    loop {
        let chunk = await_cancellable(task, async { response.chunk().await.map_err(err) }).await?;
        let Some(chunk) = chunk else {
            break;
        };
        size += chunk.len() as u64;
        if size > asset.size {
            return Err("下载大小超过清单声明".into());
        }
        file.write_all(&chunk).map_err(err)?;
        let _ = task.update(
            (size * 75 / asset.size) as u8,
            "download",
            &format!(
                "正在下载 {}：{} / {} MB",
                asset.id,
                size / 1048576,
                asset.size / 1048576
            ),
        );
    }
    file.sync_all().map_err(err)?;
    drop(file);
    verify_file(target, asset)
}
fn archive_path(name: &str, expected_root: &str) -> Result<PathBuf, String> {
    // Reject Windows aliases, ADS, traversal, absolute paths and reserved names,
    // independently of the host platform used to run tests.
    let pieces: Vec<_> = name.trim_end_matches('/').split('/').collect();
    if pieces.len() < 2
        || pieces[0] != expected_root
        || name.contains('\\')
        || pieces.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || p.ends_with(['.', ' '])
                || p.chars().any(|c| c < ' ' || ":<>\"|?*".contains(c))
                || {
                    let base = p.split('.').next().unwrap_or("").to_ascii_uppercase();
                    matches!(base.as_str(), "CON" | "NUL" | "AUX" | "PRN")
                        || (base.len() == 4
                            && (base.starts_with("COM") || base.starts_with("LPT"))
                            && base.as_bytes()[3].is_ascii_digit())
                }
        })
    {
        return Err(format!("压缩包路径不安全：{name}"));
    }
    Ok(pieces.iter().collect())
}
fn extract(zip: &Path, stage: &Path, name: &str, task: &TaskHandle) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(fs::File::open(zip).map_err(err)?).map_err(err)?;
    if archive.len() > 100_000 {
        return Err("资源文件数量过多".into());
    }
    let count = archive.len();
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        cancelled(task)?;
        let mut entry = archive.by_index(i).map_err(err)?;
        let relative = archive_path(entry.name(), name)?;
        if !seen.insert(relative.to_string_lossy().to_lowercase())
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("资源包含重复文件或链接".into());
        }
        total = total.checked_add(entry.size()).ok_or("资源体积溢出")?;
        if total > 2 * 1024 * 1024 * 1024 || entry.size() > 256 * 1024 * 1024 {
            return Err("解压体积超过限制".into());
        }
        let path = stage.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(path).map_err(err)?;
        } else {
            fs::create_dir_all(path.parent().ok_or("无效路径")?).map_err(err)?;
            let mut output = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .map_err(err)?;
            let expected = entry.size();
            let copied =
                std::io::copy(&mut (&mut entry).take(expected + 1), &mut output).map_err(err)?;
            if copied != expected {
                return Err("解压大小不符".into());
            }
        }
        if i % 100 == 0 {
            let _ = task.update(
                80 + (i * 15 / count.max(1)) as u8,
                "extract",
                "正在校验并安装 Mod",
            );
        }
    }
    Ok(())
}
fn reject_links(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors().filter(|p| p.exists()) {
        let m = fs::symlink_metadata(ancestor).map_err(err)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if m.file_attributes() & 0x400 != 0 {
                return Err("安装路径包含重解析点，请使用真实目录".into());
            }
        }
        if m.file_type().is_symlink() {
            return Err("安装路径包含符号链接".into());
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn install_mod_resource(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    edition: String,
    resource_id: String,
    local_file: Option<String>,
) -> Result<InstallResult, String> {
    let shared = state.inner().clone();
    let _lease = BuildLease::acquire(&shared)?;
    let c = catalog()?;
    let a = c
        .assets
        .iter()
        .find(|a| a.id == resource_id)
        .ok_or("未知资源")?
        .clone();
    let parent = if a.id == "processor" {
        tools_root(&app)?
    } else {
        let game = game_root(&shared, &edition)?;
        if Some(game_version(&game)?) != a.game_data_version {
            return Err("成品与当前游戏数据版本不一致，请等待适配资源或使用独立生成器".into());
        }
        game.join("mods")
    };
    reject_links(&parent)?;
    fs::create_dir_all(&parent).map_err(err)?;
    let destination = if a.id == "processor" {
        processor_path(&parent, &a).parent().unwrap().to_path_buf()
    } else {
        parent.join(&a.id)
    };
    if destination.exists() {
        return Err(format!(
            "目标已存在，保留现有文件：{}。如需重装，请先将该目录移出安装位置。",
            destination.display()
        ));
    }
    let task = shared
        .tasks()
        .begin(
            TaskRequest::new("mod-resource-install")
                .for_subject(format!("{edition}:{}", a.id))
                .with_conflict_key("audio-mod-build")
                .non_retryable()
                .with_initial_status("download", "正在准备资源安装"),
        )
        .map_err(err)?;
    let id = task.task_id();
    let stage = parent.join(format!(".d2rhub-resource-{}", uuid::Uuid::new_v4()));
    let result: Result<PathBuf, String> = async {
        fs::create_dir(&stage).map_err(err)?;
        let payload = stage.join("payload");
        if let Some(local) = local_file {
            verify_file(Path::new(&local), &a)?;
            fs::copy(local, &payload).map_err(err)?;
            verify_file(&payload, &a)?;
        } else {
            download(&a, &payload, &task).await?;
        }
        cancelled(&task)?;
        let _ = task.update(80, "verify", "下载完成，正在校验资源");
        let output = stage.join(&a.id);
        if a.id == "processor" {
            fs::create_dir(&output).map_err(err)?;
            let executable = output.join("d2r-audio-mod.exe");
            fs::rename(&payload, &executable).map_err(err)?;
            if probe_version(&app, &executable).await.as_deref() != Some(&a.version) {
                return Err("加工器实际版本与清单不一致".into());
            }
        } else {
            extract(&payload, &stage, &a.id, &task)?;
            let info =
                crate::lightweight_mod::inspect(&output, &a.id)?.ok_or("缺少 Hub Mod 身份清单")?;
            if Some(info.profile) != a.profile {
                return Err("Mod 方案不匹配".into());
            }
            let version = fs::read_to_string(
                output.join(format!("{}.mpq/data/global/dataversionbuild.txt", a.id)),
            )
            .map_err(err)?;
            if Some(version.trim().to_owned()) != a.game_data_version {
                return Err("Mod 数据版本不匹配".into());
            }
        }
        cancelled(&task)?;
        reject_links(&parent)?;
        // A same-volume rename publishes the complete directory; never overwrite.
        durable_fs::durable_rename(&output, &destination).map_err(err)?;
        if a.id == "processor" {
            let _ = save_catalog(&parent.join("installed.json"), &c);
        }
        Ok(destination.clone())
    }
    .await;
    // stage is always a newly created UUID child of the checked destination parent.
    if stage.exists() && reject_links(&stage).is_ok() {
        let _ = fs::remove_dir_all(&stage);
    }
    match result {
        Ok(path) => {
            let _ = task.succeed("资源已安装，可在 Mod 管理中使用");
            Ok(InstallResult {
                path: path.to_string_lossy().into_owned(),
                task_id: id,
            })
        }
        Err(e) => {
            if task.cancellation_requested() {
                let _ = task.cancelled(&e);
            } else {
                let _ = task.fail("resource-install-failed", &e);
            }
            Err(e)
        }
    }
}
#[tauri::command]
pub fn open_mod_processor_directory(app: tauri::AppHandle) -> Result<(), String> {
    let root = tools_root(&app)?;
    fs::create_dir_all(&root).map_err(err)?;
    open::that(root).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::task_runtime::TaskRuntime;
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("hub-resource-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn late_refresh_cannot_downgrade_memory_or_restart_cache() {
        let scratch = Scratch::new();
        let path = scratch.0.join("catalog.json");
        let cache = Mutex::new(None);
        let base: Catalog = serde_json::from_str(EMBEDDED).unwrap();
        let mut older = base.clone();
        older.revision += 1;
        let mut newer = base.clone();
        newer.revision += 2;
        // Both network requests start from base; the newer response completes first.
        publish_catalog(&cache, &path, base.clone(), false).unwrap();
        std::thread::scope(|scope| {
            let (sent, received) = std::sync::mpsc::channel();
            let cache = &cache;
            let path = &path;
            let newer = &newer;
            scope.spawn(move || {
                let (_, warning) = publish_catalog(cache, path, newer.clone(), true).unwrap();
                assert!(warning.is_none());
                sent.send(()).unwrap();
            });
            scope.spawn(move || {
                received.recv().unwrap();
                let (selected, warning) = publish_catalog(cache, path, older, true).unwrap();
                assert_eq!(selected.revision, base.revision + 2);
                assert!(warning.unwrap().contains("较旧"));
            });
        });
        assert_eq!(
            cache.lock().unwrap().as_ref().unwrap().revision,
            newer.revision
        );
        let disk: Catalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(disk.revision, newer.revision);
        let restarted = Mutex::new(None);
        let (selected, _) = publish_catalog(&restarted, &path, base, false).unwrap();
        assert_eq!(selected.revision, newer.revision);
    }

    #[test]
    fn cache_failure_retains_newest_catalog_and_later_refresh_repairs_disk() {
        let scratch = Scratch::new();
        let parent = scratch.0.join("blocked");
        fs::write(&parent, b"not a directory").unwrap();
        let path = parent.join("catalog.json");
        let cache = Mutex::new(None);
        let older: Catalog = serde_json::from_str(EMBEDDED).unwrap();
        let mut newer = older.clone();
        newer.revision += 1;
        let (selected, warning) = publish_catalog(&cache, &path, newer.clone(), true).unwrap();
        assert_eq!(selected.revision, newer.revision);
        assert!(warning.unwrap().contains("无法缓存"));
        fs::remove_file(parent).unwrap();
        publish_catalog(&cache, &path, older, true).unwrap();
        let disk: Catalog = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(disk.revision, newer.revision);
    }

    #[test]
    fn cancellation_drops_a_request_stalled_before_response_headers() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct Dropped<'a>(&'a AtomicBool);
        impl Drop for Dropped<'_> {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        executor.block_on(async {
            let runtime = TaskRuntime::new(4);
            let task = runtime
                .begin(TaskRequest::new("mod-resource-install"))
                .unwrap();
            let dropped = AtomicBool::new(false);
            let operation = async {
                let _guard = Dropped(&dropped);
                std::future::pending::<Result<(), String>>().await
            };
            let (result, ()) = tokio::join!(
                tokio::time::timeout(Duration::from_secs(2), await_cancellable(&task, operation)),
                async {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    runtime.request_cancel(task.task_id()).unwrap();
                }
            );
            assert_eq!(result.unwrap().unwrap_err(), "下载或安装已取消");
            assert!(dropped.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn cancellable_wait_preserves_requests_and_errors_and_skips_cancelled_work() {
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        executor.block_on(async {
            let runtime = TaskRuntime::new(4);
            let task = runtime
                .begin(TaskRequest::new("mod-resource-install"))
                .unwrap();
            let result = tokio::time::timeout(
                Duration::from_secs(2),
                await_cancellable(&task, async {
                    // Longer than a cancellation tick: restarting this future would never finish.
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    Ok(42)
                }),
            )
            .await
            .unwrap();
            assert_eq!(result.unwrap(), 42);
            assert_eq!(
                await_cancellable::<()>(&task, async { Err("network error".into()) })
                    .await
                    .unwrap_err(),
                "network error"
            );
            runtime.request_cancel(task.task_id()).unwrap();
            let polled = std::cell::Cell::new(false);
            let result = await_cancellable(&task, async {
                polled.set(true);
                Ok(())
            })
            .await;
            assert_eq!(result.unwrap_err(), "下载或安装已取消");
            assert!(!polled.get());
        });
    }

    #[test]
    fn rejects_tampered_payload_and_cancelled_extraction() {
        let scratch = Scratch::new();
        let path = scratch.0.join("file");
        fs::write(&path, b"original").unwrap();
        let mut a = processor_asset(&serde_json::from_str::<Catalog>(EMBEDDED).unwrap()).clone();
        a.size = 8;
        a.sha256 = digest(&path).unwrap();
        verify_file(&path, &a).unwrap();
        fs::write(&path, b"modified").unwrap();
        assert!(verify_file(&path, &a).is_err());
        let zip = scratch.0.join("test.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
        writer
            .start_file("LiteHub/file", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"data").unwrap();
        writer.finish().unwrap();
        let runtime = TaskRuntime::new(4);
        let task = runtime
            .begin(TaskRequest::new("mod-resource-install"))
            .unwrap();
        runtime.request_cancel(task.task_id()).unwrap();
        assert!(extract(&zip, &scratch.0, "LiteHub", &task).is_err());
        assert!(!scratch.0.join("LiteHub").exists());
    }
    #[test]
    fn extraction_rejects_case_collisions_and_keeps_files_inside_stage() {
        let scratch = Scratch::new();
        let zip = scratch.0.join("test.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
        for name in ["LiteHub/File", "LiteHub/file"] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"data").unwrap();
        }
        writer.finish().unwrap();
        let runtime = TaskRuntime::new(4);
        let task = runtime
            .begin(TaskRequest::new("mod-resource-install"))
            .unwrap();
        assert!(extract(&zip, &scratch.0, "LiteHub", &task)
            .unwrap_err()
            .contains("重复文件"));
        assert_eq!(fs::read(scratch.0.join("LiteHub/File")).unwrap(), b"data");
    }
    #[test]
    #[ignore = "Set D2RHUB_RESOURCE_ASSETS to the release staging directory"]
    fn release_packages_pass_hub_validation() {
        let assets = PathBuf::from(std::env::var("D2RHUB_RESOURCE_ASSETS").unwrap());
        let catalog: Catalog = serde_json::from_str(EMBEDDED).unwrap();
        let scratch = Scratch::new();
        for a in &catalog.assets {
            let file = assets.join(a.url.rsplit('/').next().unwrap());
            verify_file(&file, a).unwrap();
            if a.id == "processor" {
                continue;
            }
            let runtime = TaskRuntime::new(4);
            let task = runtime
                .begin(TaskRequest::new("mod-resource-install"))
                .unwrap();
            extract(&file, &scratch.0, &a.id, &task).unwrap();
            let metadata = crate::lightweight_mod::inspect(&scratch.0.join(&a.id), &a.id)
                .unwrap()
                .unwrap();
            assert_eq!(Some(metadata.profile), a.profile);
            assert_eq!(
                metadata.arguments,
                format!("-mod {} -txt -assettestmode 1", a.id)
            );
        }
    }
    #[test]
    fn packaged_catalog_is_complete_and_trusted() {
        let c: Catalog = serde_json::from_str(EMBEDDED).unwrap();
        validate_catalog(&c).unwrap();
        let mut bad = c.clone();
        bad.assets[0].url = "https://github.com/attacker/releases/tool.exe".into();
        assert!(validate_catalog(&bad).is_err());
        bad = c.clone();
        bad.assets[1].id = "processor".into();
        assert!(validate_catalog(&bad).is_err());
        bad = c;
        bad.channel = "future-incompatible".into();
        assert!(validate_catalog(&bad).is_err());
    }
    #[test]
    fn archive_rejects_windows_traversal_and_aliases() {
        for path in [
            "../evil",
            "/LiteHub/file",
            "LiteHub/../evil",
            "LiteHub/C:/evil",
            "LiteHub/a:stream",
            "LiteHub/CON.txt",
            "LiteHub/a./b",
            "LiteHub\\evil",
            "BoHub/file",
            "LiteHub//file",
        ] {
            assert!(archive_path(path, "LiteHub").is_err(), "{path}");
        }
        assert!(archive_path("LiteHub/LiteHub.mpq/data/test.json", "LiteHub").is_ok());
    }
    #[test]
    fn build_version_uses_active_row_and_named_column() {
        assert_eq!(
            parse_game_version("Version!STRING:0|Active!DEC:1\n3.3.90000|0\n3.3.93854|1").unwrap(),
            "93854"
        );
        assert!(parse_game_version("Version!STRING:0|Active!DEC:1\nunknown|1").is_err());
    }
}
