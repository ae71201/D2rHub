//! Edit only Act IV's six additional waypoint cells in supported local Mods.
use crate::{infrastructure::durable_fs, resource_install::no_links, state::SharedState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const TABLE: &str = "data/global/excel/actinfo.txt";
const MANIFEST: &str = "enhancement-manifest.json";
const MARKER: &str = "d2rhub-waypoints.json";
const JOURNAL: &str = ".d2rhub-waypoints-transaction.json";

#[derive(Clone, Deserialize, Serialize)]
pub struct WaypointOption {
    id: String,
    act: u8,
    label_zh: String,
    label_en: String,
    string_key: String,
}
#[derive(Deserialize)]
struct Catalog {
    defaults: Vec<String>,
    options: Vec<WaypointOption>,
}
fn catalog() -> Result<Catalog, String> {
    serde_json::from_str(include_str!("../resources/waypoint-catalog.json")).map_err(error)
}
#[derive(Serialize)]
pub struct WaypointConfig {
    selected: Vec<String>,
    defaults: Vec<String>,
    options: Vec<WaypointOption>,
    etag: String,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    table: String,
    manifest: String,
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn small_read(path: &Path) -> Result<Vec<u8>, String> {
    no_links(path)?;
    if fs::metadata(path).map_err(error)?.len() > 32 * 1024 * 1024 {
        return Err("传送配置文件过大".into());
    }
    fs::read(path).map_err(error)
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        no_links(path)?;
    }
    let temporary = path.with_file_name(format!(".waypoints-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(error)?;
        f.write_all(bytes).map_err(error)?;
        f.sync_all().map_err(error)?;
        drop(f);
        if path.exists() {
            durable_fs::durable_sibling_replace(&temporary, path).map_err(error)?;
        } else {
            durable_fs::durable_sibling_rename(&temporary, path).map_err(error)?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Finds stable column keys, preserving all unrelated acts, fields and newlines.
fn transform_table(
    text: &str,
    selected: Option<&[String]>,
) -> Result<(String, Vec<String>), String> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let header = lines
        .first()
        .ok_or("传送表为空")?
        .trim_start_matches('\u{feff}')
        .trim_end_matches(['\r', '\n']);
    let keys: Vec<_> = header.split('\t').collect();
    let column = |name: &str| -> Result<usize, String> {
        let positions: Vec<_> = keys
            .iter()
            .enumerate()
            .filter(|(_, k)| **k == name)
            .map(|(i, _)| i)
            .collect();
        if positions.len() != 1 {
            return Err(format!("传送表缺少或重复列：{name}"));
        }
        Ok(positions[0])
    };
    let act = column("act")?;
    let slots = (4..=9)
        .map(|n| column(&format!("waypoint{n}")))
        .collect::<Result<Vec<_>, _>>()?;
    if selected.is_some_and(|s| s.len() != 6) {
        return Err("必须提供六个传送位置".into());
    }
    let mut count = 0;
    let mut current = Vec::new();
    let mut output = String::new();
    for (i, line) in lines.iter().enumerate() {
        let body = line.trim_end_matches(['\r', '\n']);
        let mut fields: Vec<_> = body.split('\t').map(str::to_string).collect();
        if i > 0 && fields.get(act).is_some_and(|v| v == "4") {
            count += 1;
            if fields.len() != keys.len() {
                return Err("第四幕传送行列数不匹配".into());
            }
            current = slots.iter().map(|&i| fields[i].clone()).collect();
            if let Some(values) = selected {
                for (&column, value) in slots.iter().zip(values) {
                    fields[column] = value.clone();
                }
                output.push_str(&fields.join("\t"));
                output.push_str(&line[body.len()..]);
                continue;
            }
        }
        output.push_str(line);
    }
    if count != 1 {
        return Err("传送表必须包含唯一第四幕行".into());
    }
    Ok((output, current))
}
fn validate_selected(selected: &[String], c: &Catalog) -> Result<(), String> {
    if selected.len() != 6 {
        return Err("请配置六个传送位置".into());
    }
    let mut used = BTreeSet::new();
    for id in selected.iter().filter(|s| !s.is_empty()) {
        if !c.options.iter().any(|o| o.id == *id) {
            return Err(format!("不支持的传送目的地：{id}"));
        }
        if !used.insert(id) {
            return Err("新增传送位置不能重复；不使用的位置请选择空白".into());
        }
    }
    Ok(())
}

fn mod_root(state: &SharedState, edition: &str, name: &str) -> Result<PathBuf, String> {
    if !matches!(name, "LiteHub" | "BoHub") {
        return Err("此功能仅支持 LiteHub 和 BoHub".into());
    }
    let config = state.configuration().snapshot().ok_or("尚未配置游戏目录")?;
    let game = match edition {
        "CN" => config.cn_game_path,
        "Global" => config.global_game_path,
        _ => return Err("未知客户端版本".into()),
    };
    if game.trim().is_empty() {
        return Err("尚未配置游戏目录".into());
    }
    let mods = Path::new(game.trim()).join("mods");
    no_links(&mods)?;
    let root = mods.join(name);
    for path in [
        &root,
        &root.join(format!("{name}.mpq")),
        &root.join(format!("{name}.mpq/data")),
        &root.join(format!("{name}.mpq/data/global")),
        &root.join(format!("{name}.mpq/data/global/excel")),
    ] {
        no_links(path)?;
        if !path.is_dir() {
            return Err("请先解压并安装支持快捷传送的 Mod".into());
        }
    }
    Ok(root)
}
fn ensure_closed() -> Result<(), String> {
    if sysinfo::System::new_all()
        .processes()
        .values()
        .any(|p| p.name().to_string_lossy().eq_ignore_ascii_case("D2R.exe"))
    {
        return Err("请先关闭游戏，再保存传送列表".into());
    }
    Ok(())
}
fn table_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{name}.mpq")).join(TABLE)
}
fn recover(root: &Path, name: &str) -> Result<(), String> {
    if !root.join(JOURNAL).exists() {
        return Ok(());
    }
    ensure_closed()?;
    let journal: Journal =
        serde_json::from_slice(&small_read(&root.join(JOURNAL))?).map_err(error)?;
    // All destination paths are fixed here; the journal cannot supply a path.
    atomic_write(&table_path(root, name), journal.table.as_bytes())?;
    atomic_write(&root.join(MANIFEST), journal.manifest.as_bytes())?;
    fs::remove_file(root.join(JOURNAL)).map_err(error)
}
fn read_config(root: &Path, name: &str) -> Result<Option<WaypointConfig>, String> {
    if !root.join(MARKER).exists() {
        return Ok(None);
    }
    let marker: Value = serde_json::from_slice(&small_read(&root.join(MARKER))?).map_err(error)?;
    if marker["revision"] != 1 || marker["feature"] != "act4_waypoints" {
        return Err("不支持的快捷传送配置版本".into());
    }
    recover(root, name)?;
    let bytes = small_read(&table_path(root, name))?;
    let text = std::str::from_utf8(&bytes).map_err(error)?;
    let (_, selected) = transform_table(text, None)?;
    let c = catalog()?;
    validate_selected(&selected, &c)?;
    Ok(Some(WaypointConfig {
        selected,
        defaults: c.defaults,
        options: c.options,
        etag: hash(&bytes),
    }))
}

#[tauri::command]
pub fn get_mod_waypoints(
    state: tauri::State<'_, SharedState>,
    edition: String,
    mod_name: String,
) -> Result<Option<WaypointConfig>, String> {
    let _lease = state.mod_mutations().try_acquire()?;
    let root = mod_root(state.inner(), &edition, &mod_name)?;
    read_config(&root, &mod_name)
}

fn save(
    root: &Path,
    name: &str,
    selected: &[String],
    etag: &str,
) -> Result<WaypointConfig, String> {
    let before = read_config(root, name)?.ok_or("请先安装支持快捷传送的新版 Mod")?;
    if before.etag != etag {
        return Err("传送列表已被修改，请重新加载后保存".into());
    }
    validate_selected(selected, &catalog()?)?;
    let path = table_path(root, name);
    let old = String::from_utf8(small_read(&path)?).map_err(error)?;
    let (next, _) = transform_table(&old, Some(selected))?;
    let old_manifest = String::from_utf8(small_read(&root.join(MANIFEST))?).map_err(error)?;
    let mut manifest: Value = serde_json::from_str(&old_manifest).map_err(error)?;
    if manifest["mod_name"] != name {
        return Err("Mod 清单名称不匹配".into());
    }
    let entry = manifest["files"]
        .as_object_mut()
        .and_then(|v| v.get_mut(TABLE))
        .ok_or("清单缺少传送表校验值")?;
    if entry["sha256"] != hash(old.as_bytes()) {
        return Err("传送表校验值不一致，请先检查本地修改".into());
    }
    let prior_size = entry["bytes"].as_u64().ok_or("传送表大小记录无效")?;
    *entry = serde_json::json!({"sha256":hash(next.as_bytes()),"bytes":next.len()});
    let total = manifest["generated_bytes"]
        .as_u64()
        .ok_or("Mod 大小记录无效")?;
    manifest["generated_bytes"] = serde_json::json!(
        total.checked_sub(prior_size).ok_or("Mod 大小记录无效")? + next.len() as u64
    );
    manifest["features"]["act4_waypoints"]["selected"] = serde_json::json!(selected);
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(error)?;
    let journal = Journal {
        table: old,
        manifest: old_manifest,
    };
    // A persistent rollback record covers a crash between the two replacements.
    atomic_write(
        &root.join(JOURNAL),
        &serde_json::to_vec(&journal).map_err(error)?,
    )?;
    let result = (|| {
        atomic_write(&path, next.as_bytes())?;
        atomic_write(&root.join(MANIFEST), &manifest_bytes)?;
        if small_read(&path)? != next.as_bytes()
            || small_read(&root.join(MANIFEST))? != manifest_bytes
        {
            return Err("传送配置写入校验失败".into());
        }
        fs::remove_file(root.join(JOURNAL)).map_err(error)?;
        read_config(root, name)?.ok_or_else(|| "快捷传送配置丢失".into())
    })();
    if result.is_err() {
        recover(root, name)?;
    }
    result
}

#[tauri::command]
pub fn save_mod_waypoints(
    state: tauri::State<'_, SharedState>,
    edition: String,
    mod_name: String,
    selected: Vec<String>,
    etag: String,
) -> Result<WaypointConfig, String> {
    let _lease = state.mod_mutations().try_acquire()?;
    let root = mod_root(state.inner(), &edition, &mod_name)?;
    ensure_closed()?;
    save(&root, &mod_name, &selected, &etag)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> String {
        let mut s="act\ttown\twaypoint1\twaypoint2\twaypoint3\twaypoint4\twaypoint5\twaypoint6\twaypoint7\twaypoint8\twaypoint9\tother\r\n".to_string();
        s.push_str("1\tfirst\ta\tb\tc\td\te\tf\tg\th\ti\tx\r\n4\ttown\tkeep1\tkeep2\tkeep3\t\t\t\t\t\t\ttail\r\n5\tlast\ta\tb\tc\td\te\tf\tg\th\ti\ty\r\n");
        s
    }
    #[test]
    fn only_six_cells_change_and_defaults_keep_requested_order() {
        let original = fixture();
        let c = catalog().unwrap();
        let (updated, _) = transform_table(&original, Some(&c.defaults)).unwrap();
        assert_eq!(transform_table(&updated, None).unwrap().1, c.defaults);
        assert!(updated.contains(
            "4\ttown\tkeep1\tkeep2\tkeep3\tAct 3 - Travincal\tAct 2 - Valley of the Kings"
        ));
        for (a, b) in original.lines().zip(updated.lines()) {
            if !a.starts_with("4\t") {
                assert_eq!(a, b);
            }
        }
        assert_eq!(updated.matches("\r\n").count(), 4);
        assert_eq!(
            transform_table(&updated, Some(&vec![String::new(); 6]))
                .unwrap()
                .0,
            original
        );
    }
    #[test]
    fn rejects_bad_tables_unknown_destinations_and_duplicates() {
        assert!(transform_table(&fixture().replace("waypoint9", "waypoint8"), None).is_err());
        assert!(transform_table(&fixture().replace("5\tlast", "4\tlast"), None).is_err());
        let c = catalog().unwrap();
        assert!(validate_selected(&vec!["../outside".into(); 6], &c).is_err());
        assert!(validate_selected(&vec![c.defaults[0].clone(); 6], &c).is_err());
        assert!(validate_selected(&vec![String::new(); 6], &c).is_ok());
    }
    #[test]
    fn save_updates_integrity_and_rejects_stale_writes() {
        let root = std::env::temp_dir().join(format!("hub-waypoints-{}", uuid::Uuid::new_v4()));
        let path = table_path(&root, "LiteHub");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = fixture();
        fs::write(&path, &original).unwrap();
        fs::write(
            root.join(MARKER),
            r#"{"revision":1,"feature":"act4_waypoints"}"#,
        )
        .unwrap();
        let manifest = serde_json::json!({"mod_name":"LiteHub","files":{TABLE:{"sha256":hash(original.as_bytes()),"bytes":original.len()}},"generated_bytes":original.len(),"features":{}});
        fs::write(root.join(MANIFEST), serde_json::to_vec(&manifest).unwrap()).unwrap();
        let c = read_config(&root, "LiteHub").unwrap().unwrap();
        let next = save(&root, "LiteHub", &c.defaults, &c.etag).unwrap();
        assert_eq!(next.selected, c.defaults);
        assert_ne!(next.etag, c.etag);
        assert!(save(&root, "LiteHub", &vec![String::new(); 6], &c.etag).is_err());
        let after: Value = serde_json::from_slice(&fs::read(root.join(MANIFEST)).unwrap()).unwrap();
        assert_eq!(after["files"][TABLE]["sha256"], next.etag);
        assert!(!root.join(JOURNAL).exists());
        fs::remove_dir_all(root).unwrap();
    }
}
