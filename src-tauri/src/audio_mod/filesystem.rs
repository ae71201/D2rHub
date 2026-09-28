//! Mod directory boundary checks and durable filesystem operations.
//! Callers own the application mutation lease; this adapter rejects links and
//! reparse points before traversing or mutating any existing Mod tree.
use crate::infrastructure::durable_fs;
use std::path::{Path, PathBuf};

pub(super) struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    pub(super) fn create(path: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&path)
            .map_err(|error| format!("创建临时目录失败 {}: {error}", path.display()))?;
        Ok(Self(path))
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(super) fn find_existing_mod_name(
    mods_directory: &Path,
    candidate: &str,
) -> Result<Option<String>, String> {
    let entries = std::fs::read_dir(mods_directory)
        .map_err(|error| format!("读取 mods 目录失败: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("读取 mods 目录项失败: {error}"))?;
        let existing_name = entry.file_name().to_string_lossy().into_owned();
        if existing_name.eq_ignore_ascii_case(candidate) {
            return Ok(Some(existing_name));
        }
    }
    Ok(None)
}

pub(super) fn path_exists_no_follow(path: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("无法检查事务路径 {}：{error}", path.display())),
    }
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(_metadata: &std::fs::Metadata) -> bool {
    false
}

pub(super) fn canonical_safe_mods_root(mods_directory: &Path) -> Result<PathBuf, String> {
    let metadata = std::fs::symlink_metadata(mods_directory)
        .map_err(|error| format!("无法检查 mods 目录 {}：{error}", mods_directory.display()))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata_is_reparse_point(&metadata)
    {
        return Err("mods 目录不能是符号链接、联接点或重解析点".to_string());
    }
    std::fs::canonicalize(mods_directory)
        .map_err(|error| format!("无法规范化 mods 目录 {}：{error}", mods_directory.display()))
}

pub(super) fn ensure_safe_existing_node(
    canonical_mods: &Path,
    path: &Path,
    expect_directory: bool,
    label: &str,
) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("无法检查{label} {}：{error}", path.display()))?;
    if metadata.file_type().is_symlink() || metadata_is_reparse_point(&metadata) {
        return Err(format!("{label}不能是符号链接、联接点或重解析点"));
    }
    if expect_directory && !metadata.is_dir() {
        return Err(format!("{label}不是目录：{}", path.display()));
    }
    if !expect_directory && !metadata.is_file() {
        return Err(format!("{label}不是普通文件：{}", path.display()));
    }
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| format!("无法规范化{label} {}：{error}", path.display()))?;
    if canonical == canonical_mods || !canonical.starts_with(canonical_mods) {
        return Err(format!("{label}越过了 mods 目录边界：{}", path.display()));
    }
    Ok(())
}

pub(super) fn ensure_safe_directory_if_present(
    canonical_mods: &Path,
    path: &Path,
    label: &str,
) -> Result<bool, String> {
    if !path_exists_no_follow(path)? {
        return Ok(false);
    }
    ensure_safe_existing_node(canonical_mods, path, true, label)?;
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SafeTreeNodeKind {
    File,
    Directory,
}

pub(super) fn traverse_safe_directory_tree<F>(
    boundary_directory: &Path,
    tree_root: &Path,
    mut visitor: F,
) -> Result<(), String>
where
    F: FnMut(&Path, SafeTreeNodeKind) -> Result<(), String>,
{
    let canonical_boundary = canonical_safe_mods_root(boundary_directory)?;
    let mut pending = vec![(tree_root.to_path_buf(), false)];
    while let Some((path, children_visited)) = pending.pop() {
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("无法检查 Mod 目录树 {}：{error}", path.display()))?;
        if metadata.file_type().is_symlink() || metadata_is_reparse_point(&metadata) {
            return Err(format!(
                "Mod 目录树不能包含符号链接、联接点或重解析点：{}",
                path.display()
            ));
        }
        let kind = if metadata.is_dir() {
            SafeTreeNodeKind::Directory
        } else if metadata.is_file() {
            SafeTreeNodeKind::File
        } else {
            return Err(format!(
                "Mod 目录树包含不受支持的文件类型：{}",
                path.display()
            ));
        };
        ensure_safe_existing_node(
            &canonical_boundary,
            &path,
            kind == SafeTreeNodeKind::Directory,
            "Mod 目录树节点",
        )?;

        if kind == SafeTreeNodeKind::Directory && !children_visited {
            pending.push((path.clone(), true));
            let mut children = std::fs::read_dir(&path)
                .map_err(|error| format!("无法遍历 Mod 目录树 {}：{error}", path.display()))?
                .map(|entry| {
                    entry
                        .map(|entry| entry.path())
                        .map_err(|error| format!("无法读取 Mod 目录项：{error}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            children.sort();
            pending.extend(children.into_iter().rev().map(|child| (child, false)));
            continue;
        }

        // Directories are visited after all descendants, making directory metadata flushes
        // bottom-up. Re-checking each expanded directory also narrows the validation/mutation race.
        visitor(&path, kind)?;
    }
    Ok(())
}

pub(super) fn validate_safe_directory_tree(
    boundary_directory: &Path,
    tree_root: &Path,
) -> Result<(), String> {
    traverse_safe_directory_tree(boundary_directory, tree_root, |_path, _kind| Ok(()))
}

pub(super) fn sync_safe_directory_tree(
    mods_directory: &Path,
    tree_root: &Path,
) -> Result<(), String> {
    traverse_safe_directory_tree(mods_directory, tree_root, |path, kind| match kind {
        SafeTreeNodeKind::File => sync_regular_file(path),
        SafeTreeNodeKind::Directory => sync_directory(path),
    })?;
    let canonical_mods = canonical_safe_mods_root(mods_directory)?;
    let stage_parent = tree_root
        .parent()
        .ok_or_else(|| "Mod 目录树缺少父目录".to_string())?;
    if stage_parent != mods_directory {
        ensure_safe_existing_node(&canonical_mods, stage_parent, true, "Mod 目录树父目录")?;
        sync_directory(stage_parent)?;
    }
    sync_directory(mods_directory)
}

#[cfg(not(windows))]
fn sync_regular_file(path: &Path) -> Result<(), String> {
    std::fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("无法持久化 Mod 文件 {}：{error}", path.display()))
}

#[cfg(windows)]
fn sync_regular_file(path: &Path) -> Result<(), String> {
    // FlushFileBuffers requires a handle opened for writing on Windows. File::open creates a
    // read-only handle, which makes sync_all fail with ERROR_ACCESS_DENIED even for writable
    // staged files.
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("无法持久化 Mod 文件 {}：{error}", path.display()))
}

pub(super) fn sync_directory(path: &Path) -> Result<(), String> {
    durable_fs::sync_directory(path)
        .map_err(|error| format!("无法同步目录元数据 {}：{error}", path.display()))
}

pub(super) fn rename_directory_and_sync(
    mods_directory: &Path,
    from: &Path,
    to: &Path,
    operation: &str,
) -> Result<(), String> {
    let canonical_mods = canonical_safe_mods_root(mods_directory)?;
    ensure_safe_existing_node(&canonical_mods, from, true, operation)?;
    for (parent, label) in [
        (from.parent(), "重命名源目录的父目录"),
        (to.parent(), "重命名目标目录的父目录"),
    ] {
        let parent = parent.ok_or_else(|| format!("{operation}路径缺少父目录"))?;
        if parent != mods_directory {
            ensure_safe_existing_node(&canonical_mods, parent, true, label)?;
        }
    }
    if path_exists_no_follow(to)? {
        return Err(format!("{operation}的目标路径已存在：{}", to.display()));
    }
    durable_fs::durable_rename(from, to).map_err(|error| format!("{operation}失败：{error}"))?;
    // A cross-directory rename changes both directory entry sets. Flush the nested stage parent as
    // well as the common mods parent; target/backup/quarantine renames only need the latter.
    for parent in [from.parent(), to.parent()].into_iter().flatten() {
        if parent != mods_directory && path_exists_no_follow(parent)? {
            sync_directory(parent)?;
        }
    }
    sync_directory(mods_directory)
}

pub(super) fn remove_transaction_directory(
    mods_directory: &Path,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    if !path_exists_no_follow(path)? {
        return Ok(());
    }
    let canonical_mods = canonical_safe_mods_root(mods_directory)?;
    ensure_safe_existing_node(&canonical_mods, path, true, label)?;
    validate_safe_directory_tree(mods_directory, path)
        .map_err(|error| format!("拒绝清理不安全的{label}：{error}"))?;
    std::fs::remove_dir_all(path)
        .map_err(|error| format!("清理{label}失败 {}：{error}", path.display()))?;
    sync_directory(mods_directory)
}
