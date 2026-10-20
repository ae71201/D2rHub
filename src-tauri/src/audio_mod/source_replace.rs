//! Same-name processing retains the original directory as a permanent backup.
use super::filesystem::*;
use crate::domain::mod_arguments::generated_audio_mod_name;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

const PREFIX: &str = ".d2rhub-source-replace-";

#[derive(Serialize, Deserialize)]
struct Journal {
    name: String,
    backup: String,
    id: String,
}

pub(super) fn backup_name(name: &str, id: &str) -> String {
    format!("{name}_backup_{id}")
}

fn journal_path(root: &Path, id: &str) -> PathBuf {
    root.join(format!("{PREFIX}{id}.json"))
}

pub(super) fn commit(root: &Path, name: &str, stage: &Path, id: &str) -> Result<String, String> {
    generated_audio_mod_name(name)?;
    if id.len() != 32
        || !id.bytes().all(|b| b.is_ascii_hexdigit())
        || stage != root.join(format!(".d2rhub-source-stage-{id}")).join(name)
    {
        return Err("同名加工暂存目录无效".into());
    }
    let backup = backup_name(name, id);
    validate_safe_directory_tree(root, &root.join(name))?;
    validate_safe_directory_tree(root, stage)?;
    super::validation::validate_audio_mod_directory(root, name, stage.to_path_buf())?;
    sync_safe_directory_tree(root, stage)?;
    if path_exists_no_follow(&root.join(&backup))? {
        return Err("源 Mod 备份目录已存在，未替换".into());
    }
    let journal = Journal {
        name: name.into(),
        backup: backup.clone(),
        id: id.into(),
    };
    let path = journal_path(root, id);
    let pending = root.join(format!("{PREFIX}{id}.pending"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(&journal).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    crate::infrastructure::durable_fs::durable_sibling_rename(&pending, &path)
        .map_err(|e| e.to_string())?;
    sync_directory(root)?;
    let result: Result<(), String> = (|| {
        rename_directory_and_sync(
            root,
            &root.join(name),
            &root.join(&backup),
            "保留源 Mod 备份",
        )?;
        rename_directory_and_sync(root, stage, &root.join(name), "安装同名 Mod")?;
        super::validation::validate_audio_mod(root, name)?;
        Ok(())
    })();
    if let Err(error) = result {
        return match recover(root) {
            Ok(()) => Err(format!(
                "同名加工未完成：{error}；原始文件保留在原目录或 {backup}"
            )),
            Err(recovery) => Err(format!(
                "{error}；恢复待处理：{recovery}；备份保留在 {backup}"
            )),
        };
    }
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    sync_directory(root)?;
    Ok(backup)
}

pub(super) fn recover(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let canonical = canonical_safe_mods_root(root)?;
    for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let filename = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = filename
            .strip_prefix(PREFIX)
            .and_then(|s| s.strip_suffix(".json"))
        else {
            continue;
        };
        if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("同名加工恢复记录标识无效".into());
        }
        ensure_safe_existing_node(&canonical, &entry.path(), false, "同名加工恢复记录")?;
        if entry.metadata().map_err(|e| e.to_string())?.len() > 4096 {
            return Err("同名加工恢复记录过大".into());
        }
        let journal: Journal =
            serde_json::from_slice(&std::fs::read(entry.path()).map_err(|e| e.to_string())?)
                .map_err(|e| format!("同名加工恢复记录损坏：{e}"))?;
        generated_audio_mod_name(&journal.name)?;
        if journal.id != id || journal.backup != backup_name(&journal.name, id) {
            return Err("同名加工恢复路径无效".into());
        }
        let target = root.join(&journal.name);
        let backup = root.join(&journal.backup);
        if path_exists_no_follow(&backup)? {
            validate_safe_directory_tree(root, &backup)?;
            if path_exists_no_follow(&target)? {
                if super::validation::validate_audio_mod(root, &journal.name).is_err() {
                    let quarantine = root.join(format!(".d2rhub-source-failed-{id}"));
                    rename_directory_and_sync(root, &target, &quarantine, "保留未完成加工结果")?;
                    rename_directory_and_sync(root, &backup, &target, "恢复源 Mod")?;
                }
            } else {
                rename_directory_and_sync(root, &backup, &target, "恢复源 Mod")?;
            }
        } else if !path_exists_no_follow(&target)? {
            return Err("同名加工的原目录与备份均缺失，请检查磁盘".into());
        }
        std::fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
        sync_directory(root)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_output_never_renames_the_source() {
        let root =
            std::env::temp_dir().join(format!("source-replace-test-{}", uuid::Uuid::new_v4()));
        let id = uuid::Uuid::new_v4().simple().to_string();
        let stage = root
            .join(format!(".d2rhub-source-stage-{id}"))
            .join("MYMOD");
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::create_dir_all(root.join("MYMOD")).unwrap();
        std::fs::write(root.join("MYMOD/original.txt"), b"keep").unwrap();
        assert!(commit(&root, "MYMOD", &stage, &id).is_err());
        assert_eq!(
            std::fs::read(root.join("MYMOD/original.txt")).unwrap(),
            b"keep"
        );
        assert!(!root.join(backup_name("MYMOD", &id)).exists());
        assert!(!journal_path(&root, &id).exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_installed_result_is_quarantined_and_original_restored() {
        let root =
            std::env::temp_dir().join(format!("source-replace-test-{}", uuid::Uuid::new_v4()));
        let id = uuid::Uuid::new_v4().simple().to_string();
        let backup = backup_name("MYMOD", &id);
        std::fs::create_dir_all(root.join(&backup)).unwrap();
        std::fs::create_dir_all(root.join("MYMOD")).unwrap();
        std::fs::write(root.join(&backup).join("original.txt"), b"keep").unwrap();
        std::fs::write(root.join("MYMOD/broken.txt"), b"preserve for review").unwrap();
        std::fs::write(
            journal_path(&root, &id),
            serde_json::to_vec(&Journal {
                name: "MYMOD".into(),
                backup,
                id: id.clone(),
            })
            .unwrap(),
        )
        .unwrap();
        recover(&root).unwrap();
        assert_eq!(
            std::fs::read(root.join("MYMOD/original.txt")).unwrap(),
            b"keep"
        );
        assert!(root
            .join(format!(".d2rhub-source-failed-{id}/broken.txt"))
            .exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn interrupted_after_backup_restores_original_without_deleting_payload() {
        let root =
            std::env::temp_dir().join(format!("source-replace-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let id = uuid::Uuid::new_v4().simple().to_string();
        let backup = backup_name("MYMOD", &id);
        std::fs::create_dir(root.join(&backup)).unwrap();
        std::fs::write(root.join(&backup).join("original.txt"), b"original").unwrap();
        std::fs::write(
            journal_path(&root, &id),
            serde_json::to_vec(&Journal {
                name: "MYMOD".into(),
                backup,
                id: id.clone(),
            })
            .unwrap(),
        )
        .unwrap();
        recover(&root).unwrap();
        assert_eq!(
            std::fs::read(root.join("MYMOD/original.txt")).unwrap(),
            b"original"
        );
        assert!(!journal_path(&root, &id).exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
