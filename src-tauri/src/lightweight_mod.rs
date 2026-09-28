//! Recognize downloadable Mod products and legacy generation manifests.
use crate::state::SharedState;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

const MANIFEST: &str = "generation-manifest.json";
const PRODUCER: &str = "d2r-native-bundled-generator";
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
#[cfg(test)]
mod tests {
    use super::*;
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
    fn fixture() -> (PathBuf, Value) {
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
        let report = serde_json::json!({"producer":PRODUCER,"mode":"bundled_rebuild","profile":"min","mod_name":"NullHub","mod_directory":root.canonicalize().unwrap(),"launch_arguments":"-mod NullHub","game_data_version":"93854","verified_output_integrity":true});
        fs::write(root.join(MANIFEST), serde_json::to_vec(&report).unwrap()).unwrap();
        (root, report)
    }

    #[test]
    fn recognizes_product_flags_and_rejects_missing_version() {
        let (root, mut report) = fixture();
        assert_eq!(
            inspect(&root, "NullHub").unwrap().unwrap().arguments,
            "-mod NullHub -txt -assettestmode 1"
        );
        assert!(inspect(&root, "NullHub").is_ok());
        report["launch_arguments"] = Value::String("-mod NullHub -assettestmode 1".into());
        fs::write(root.join(MANIFEST), serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(inspect(&root, "NullHub").is_ok());
        report["launch_arguments"] = Value::String(arguments("min", "NullHub").unwrap());
        fs::write(root.join(MANIFEST), serde_json::to_vec(&report).unwrap()).unwrap();
        assert!(inspect(&root, "NullHub").is_ok());
        let mut wrong_flags = report.clone();
        wrong_flags["launch_arguments"] =
            Value::String("-mod NullHub -txt -assettestmode 0".into());
        assert!(metadata(&wrong_flags, &root, "NullHub").is_err());
        fs::remove_file(root.join("NullHub.mpq/data/global/dataversionbuild.txt")).unwrap();
        assert!(inspect(&root, "NullHub").is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn names_alone_are_not_profile_identity() {
        let (root, _) = fixture();
        fs::remove_file(root.join(MANIFEST)).unwrap();
        assert!(inspect(&root, "NullHub").unwrap().is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
