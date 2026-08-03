use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};

use super::super::LocalEntry;

pub(super) fn read_local_directory(path: &Path) -> Result<(PathBuf, Vec<LocalEntry>)> {
    let path = path
        .canonicalize()
        .with_context(|| format!("无法访问本地目录 {}", path.display()))?;
    let mut entries = fs::read_dir(&path)
        .with_context(|| format!("读取本地目录 {} 失败", path.display()))?
        .filter_map(|entry| match entry {
            Ok(entry) => {
                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        log::debug!("读取本地文件信息失败: {error}");
                        return None;
                    }
                };
                Some(LocalEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: entry.path(),
                    is_directory: metadata.is_dir(),
                    size: metadata.len(),
                    modified_at: metadata.modified().ok(),
                })
            }
            Err(error) => {
                log::debug!("读取本地目录项失败: {error}");
                None
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok((path, entries))
}

pub(super) fn delete_local_path(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("读取本地路径 {} 失败", path.display()))?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path).with_context(|| format!("删除本地目录 {} 失败", path.display()))
    } else {
        fs::remove_file(path).with_context(|| format!("删除本地文件 {} 失败", path.display()))
    }
}
