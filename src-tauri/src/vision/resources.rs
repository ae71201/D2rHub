//! Only this small manifest is embedded. Models, dictionary metadata and ORT are downloaded.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

const ONLINE: &str =
    "https://raw.githubusercontent.com/gjy991229/D2rHub/main/resources/vision-resources-v1.json";
#[derive(Clone, Serialize, Deserialize)]
pub struct Asset {
    pub version: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub revision: u64,
    pub engine_abi: String,
    pub model: Asset,
    pub runtime: Asset,
}
pub struct Installed {
    pub model: PathBuf,
    pub runtime: PathBuf,
}
struct PartialFile(PathBuf);
impl Drop for PartialFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn validate(m: &Manifest) -> Result<(), String> {
    let runtime_version = semver::Version::parse(&m.runtime.version).map_err(err)?;
    if m.schema != 1
        || m.engine_abi != "ort-1.22"
        || runtime_version.major != 1
        || runtime_version.minor != 22
        || !runtime_version.pre.is_empty()
    {
        return Err("视觉资源与本软件推理接口不兼容".into());
    }
    for a in [&m.model, &m.runtime] {
        let u = reqwest::Url::parse(&a.url).map_err(err)?;
        let allowed = (u.host_str() == Some("www.modelscope.cn")
            && u.path().starts_with("/models/RapidAI/RapidOCR/resolve/"))
            || (u.host_str() == Some("github.com")
                && (u
                    .path()
                    .starts_with("/microsoft/onnxruntime/releases/download/")
                    || u.path().starts_with("/gjy991229/D2rHub/releases/download/")));
        if !allowed
            || u.scheme() != "https"
            || !u.username().is_empty()
            || u.password().is_some()
            || u.query().is_some()
            || u.fragment().is_some()
            || u.port().is_some()
            || a.size == 0
            || a.size > 256 * 1024 * 1024
            || a.sha256.len() != 64
            || !a.sha256.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err("视觉资源下载来源或摘要无效".into());
        }
    }
    Ok(())
}

fn adopt_newer_compatible_manifest(current: &mut Manifest, candidate: Manifest) {
    if candidate.revision > current.revision && validate(&candidate).is_ok() {
        *current = candidate;
    }
}
fn valid_file(p: &Path, a: &Asset) -> bool {
    crate::resource_install::no_links(p).is_ok()
        && fs::metadata(p).is_ok_and(|m| m.len() == a.size)
        && crate::downloads::digest(p).is_ok_and(|h| h.eq_ignore_ascii_case(&a.sha256))
}
async fn download(
    client: &reqwest::Client,
    a: &Asset,
    p: &Path,
    stop: &AtomicBool,
    progress: &impl Fn(String),
) -> Result<(), String> {
    let mut last = String::new();
    for attempt in 0..3 {
        if stop.load(Ordering::Acquire) {
            return Err("视觉资源下载已取消".into());
        }
        match download_once(client, a, p, stop, progress).await {
            Ok(()) => return Ok(()),
            Err(error) => last = error,
        }
        progress(format!("资源下载失败，重试 {}/3：{last}", attempt + 1));
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    Err(last)
}
async fn download_once(
    client: &reqwest::Client,
    a: &Asset,
    p: &Path,
    stop: &AtomicBool,
    progress: &impl Fn(String),
) -> Result<(), String> {
    if valid_file(p, a) {
        return Ok(());
    }
    let temporary = p.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
    let _partial = PartialFile(temporary.clone());
    let result = async {
        let mut response = client
            .get(&a.url)
            .send()
            .await
            .map_err(|e| format!("{e:?}"))?
            .error_for_status()
            .map_err(err)?;
        let mut f = fs::File::create(&temporary).map_err(err)?;
        let mut count = 0u64;
        let mut last_percent = 101;
        while let Some(bytes) = response.chunk().await.map_err(err)? {
            if stop.load(Ordering::Acquire) {
                return Err("视觉资源下载已取消".into());
            }
            count += bytes.len() as u64;
            if count > a.size {
                return Err("视觉资源超过清单大小".into());
            }
            f.write_all(&bytes).map_err(err)?;
            let percent = count * 100 / a.size;
            if percent != last_percent {
                progress(format!("下载 {}：{}%", a.version, percent));
                last_percent = percent;
            }
        }
        f.sync_all().map_err(err)?;
        drop(f);
        if !valid_file(&temporary, a) {
            return Err("视觉资源 SHA-256 或大小校验失败".into());
        }
        if p.exists() {
            crate::infrastructure::durable_fs::durable_sibling_replace(&temporary, p)
                .map_err(err)?;
        } else {
            fs::rename(&temporary, p).map_err(err)?;
        }
        Ok(())
    }
    .await;
    let _ = fs::remove_file(&temporary);
    result
}
pub async fn ensure(
    root: &Path,
    stop: &AtomicBool,
    progress: impl Fn(String),
) -> Result<Installed, String> {
    fs::create_dir_all(root).map_err(err)?;
    crate::resource_install::no_links(root)?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .user_agent("D2RHub-Vision/1")
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(err)?;
    let mut manifest: Manifest =
        serde_json::from_str(include_str!("../../../resources/vision-resources-v1.json"))
            .map_err(err)?;
    validate(&manifest)?;
    let cache = root.join("manifest.json");
    if let Ok(bytes) = fs::read(&cache) {
        if let Ok(cached) = serde_json::from_slice::<Manifest>(&bytes) {
            adopt_newer_compatible_manifest(&mut manifest, cached);
        }
    }
    progress("检查视觉资源版本".into());
    // A missing release index/offline host may use the pinned, verified manifest.
    let remote = async {
        let mut response = client
            .get(ONLINE)
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
            .map_err(err)?
            .error_for_status()
            .map_err(err)?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(err)? {
            if bytes.len() + chunk.len() > 65536 {
                return Err("视觉清单过大".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice::<Manifest>(&bytes).map_err(err)
    }
    .await;
    if let Ok(remote) = remote {
        // An incompatible future index must not disable the verified cache or
        // the pinned resources this client can still load.
        adopt_newer_compatible_manifest(&mut manifest, remote);
    }
    if stop.load(Ordering::Acquire) {
        return Err("视觉资源准备已取消".into());
    }
    let model = root.join(format!("{}.onnx", manifest.model.sha256));
    let archive = root.join(format!("{}.zip", manifest.runtime.sha256));
    download(&client, &manifest.model, &model, stop, &progress).await?;
    download(&client, &manifest.runtime, &archive, stop, &progress).await?;
    let runtime_dir = root.join(&manifest.runtime.sha256);
    fs::create_dir_all(&runtime_dir).map_err(err)?;
    crate::resource_install::no_links(&runtime_dir)?;
    let mut zip = zip::ZipArchive::new(fs::File::open(&archive).map_err(err)?).map_err(err)?;
    // Extract exactly the CPU runtime, never arbitrary archive paths or executables.
    let mut entry = zip
        .by_name(&format!(
            "onnxruntime-win-x64-{}/lib/onnxruntime.dll",
            manifest.runtime.version
        ))
        .map_err(err)?;
    if entry.size() > 128 * 1024 * 1024 {
        return Err("推理运行库大小无效".into());
    }
    let mut data = Vec::new();
    entry.read_to_end(&mut data).map_err(err)?;
    let runtime = runtime_dir.join("onnxruntime.dll");
    if !fs::read(&runtime).is_ok_and(|bytes| bytes == data) {
        let temp = runtime_dir.join(format!("onnxruntime.{}.part", uuid::Uuid::new_v4()));
        let _partial = PartialFile(temp.clone());
        fs::write(&temp, data).map_err(err)?;
        if runtime.exists() {
            crate::resource_install::no_links(&runtime)?;
            crate::infrastructure::durable_fs::durable_sibling_replace(&temp, &runtime)
                .map_err(|e| format!("修复运行库失败，请关闭软件后重试：{e}"))?;
        } else {
            fs::rename(temp, &runtime).map_err(err)?;
        }
    }
    crate::downloads::save_json(&cache, &manifest)?;
    progress(format!(
        "资源就绪：{} / ORT {}",
        manifest.model.version, manifest.runtime.version
    ));
    Ok(Installed { model, runtime })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_pinned_manifest_and_rejects_foreign_binary() {
        let mut m: Manifest =
            serde_json::from_str(include_str!("../../../resources/vision-resources-v1.json"))
                .unwrap();
        assert!(validate(&m).is_ok());
        m.runtime.url = "https://example.com/runtime.zip".into();
        assert!(validate(&m).is_err());
    }

    #[test]
    fn incompatible_or_invalid_updates_retain_the_compatible_manifest() {
        let mut current: Manifest =
            serde_json::from_str(include_str!("../../../resources/vision-resources-v1.json"))
                .unwrap();
        let mut candidate = current.clone();
        candidate.revision += 1;
        candidate.engine_abi = "ort-future".into();
        let revision = current.revision;
        adopt_newer_compatible_manifest(&mut current, candidate);
        assert_eq!(current.revision, revision);
        let mut candidate = current.clone();
        candidate.revision += 1;
        candidate.runtime.url = "https://example.com/runtime.zip".into();
        adopt_newer_compatible_manifest(&mut current, candidate);
        assert_eq!(current.revision, revision);
        let mut candidate = current.clone();
        candidate.revision += 1;
        adopt_newer_compatible_manifest(&mut current, candidate);
        assert_eq!(current.revision, revision + 1);
    }
}
