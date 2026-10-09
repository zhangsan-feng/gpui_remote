use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::{Context as _, Result};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::{
    application::ports::{LocalEntry, LocalWatchSource},
    domain::sftp::FileSignature,
};

pub(super) fn default_desktop_path() -> PathBuf {
    let profile = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let desktop = profile.join("Desktop");
    if desktop.is_dir() { desktop } else { profile }
}

pub(super) fn scan_local_directory(path: &Path) -> Result<(PathBuf, Vec<LocalEntry>)> {
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

pub(super) fn create_local_watcher(path: PathBuf) -> Result<LocalWatchSource> {
    let metadata =
        fs::metadata(&path).with_context(|| format!("监听本地路径不存在: {}", path.display()))?;
    let is_directory = if metadata.is_dir() {
        true
    } else if metadata.is_file() {
        false
    } else {
        anyhow::bail!("不支持监听本地路径类型: {}", path.display());
    };
    let watch_root = if is_directory {
        path.clone()
    } else {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_owned)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    let mode = if is_directory {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    let (events, receiver) = mpsc::unbounded_channel();
    let mut watcher = RecommendedWatcher::new(
        move |event: notify::Result<notify::Event>| match event {
            Ok(event) if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) => {
                let paths = event
                    .paths
                    .into_iter()
                    .filter(|candidate| {
                        if is_directory {
                            candidate == &path || candidate.starts_with(&path)
                        } else {
                            candidate == &path
                        }
                    })
                    .collect::<Vec<_>>();
                if !paths.is_empty() {
                    let _ = events.send(Ok(paths));
                }
            }
            Ok(_) => {}
            Err(error) => {
                let _ = events.send(Err(format!("本地自动上传监听异常: {error:#}")));
            }
        },
        Config::default(),
    )
    .context("创建监听器失败")?;
    watcher
        .watch(&watch_root, mode)
        .with_context(|| format!("监听路径失败 {}", watch_root.display()))?;
    Ok(LocalWatchSource {
        events: receiver,
        is_directory,
        watcher: Box::new(watcher),
    })
}

pub(super) fn read_local_signature(path: &Path) -> Result<Option<FileSignature>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("读取本地路径元数据失败 {}", path.display()));
        }
    };
    if metadata.file_type().is_symlink() {
        log::debug!("SFTP watcher 跳过符号链接: path={}", path.display());
        return Ok(None);
    }
    Ok(Some(FileSignature::new(
        metadata.is_dir(),
        metadata.len(),
        metadata.modified().ok().and_then(|time| {
            time.duration_since(UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_secs())
        }),
    )))
}
