use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use filetime::{FileTime, set_file_mtime};
use russh_sftp::{client::SftpSession, protocol::FileAttributes};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::{
    remote::join_remote_path,
    sync::{FileSignature, SyncDecision},
};

const TRANSFER_BUFFER_SIZE: usize = 64 * 1024;

async fn copy_with_progress(
    source: &mut (impl AsyncRead + Unpin),
    target: &mut (impl AsyncWrite + Unpin),
    total_size: u64,
    transferred: &mut u64,
    on_progress: &mut impl FnMut(u64, u64),
    is_cancelled: &impl Fn() -> bool,
) -> Result<()> {
    let mut buffer = vec![0; TRANSFER_BUFFER_SIZE];
    loop {
        if is_cancelled() {
            bail!("传输已取消");
        }
        let read = source.read(&mut buffer).await.context("读取传输数据失败")?;
        if read == 0 {
            break;
        }
        if is_cancelled() {
            bail!("传输已取消");
        }
        target
            .write_all(&buffer[..read])
            .await
            .context("写入传输数据失败")?;
        *transferred = transferred.saturating_add(read as u64);
        on_progress(*transferred, total_size);
    }
    Ok(())
}

pub(super) struct TransferOutcome {
    pub(super) is_directory: bool,
    pub(super) total_size: u64,
    pub(super) transferred_files: u64,
    pub(super) skipped_files: u64,
    pub(super) unchanged_entries: u64,
    pub(super) transferred_bytes: u64,
}

struct LocalTransferEntry {
    local_path: PathBuf,
    remote_path: String,
    is_directory: bool,
    signature: FileSignature,
}

struct RemoteTransferEntry {
    remote_path: String,
    local_path: PathBuf,
    is_directory: bool,
    signature: FileSignature,
}

pub(super) async fn upload_path(
    sftp: &SftpSession,
    local_path: &Path,
    remote_path: &str,
    mut on_progress: impl FnMut(u64, u64),
    is_cancelled: impl Fn() -> bool,
) -> Result<TransferOutcome> {
    let local_root = local_path.to_owned();
    let remote_root = remote_path.to_owned();
    let (entries, total_size) =
        tokio::task::spawn_blocking(move || collect_local_entries(&local_root, &remote_root))
            .await
            .context("扫描本地上传目录任务失败")??;
    let is_directory = entries.first().is_some_and(|entry| entry.is_directory);
    let mut transferred = 0_u64;
    let mut outcome = TransferOutcome {
        is_directory,
        total_size,
        transferred_files: 0,
        skipped_files: 0,
        unchanged_entries: 0,
        transferred_bytes: 0,
    };

    for entry in entries {
        if is_cancelled() {
            bail!("传输已取消");
        }
        if entry.is_directory {
            if sftp
                .try_exists(&entry.remote_path)
                .await
                .with_context(|| format!("检查远程目录 {} 失败", entry.remote_path))?
            {
                let metadata = sftp
                    .metadata(&entry.remote_path)
                    .await
                    .inspect_err(|error| {
                        log::warn!(
                            "SFTP sync metadata-error: remote={}, error={error:#}",
                            entry.remote_path
                        )
                    })
                    .with_context(|| format!("读取远程目录 {} 信息失败", entry.remote_path))?;
                if !metadata.is_dir() {
                    bail!(
                        "远程目标类型冲突：本地目录 {} 对应远程文件 {}",
                        entry.local_path.display(),
                        entry.remote_path
                    );
                }
                outcome.unchanged_entries = outcome.unchanged_entries.saturating_add(1);
                continue;
            }
            sftp.create_dir(&entry.remote_path)
                .await
                .with_context(|| format!("创建远程目录 {} 失败", entry.remote_path))?;
            continue;
        }

        if sftp
            .try_exists(&entry.remote_path)
            .await
            .with_context(|| format!("检查远程文件 {} 失败", entry.remote_path))?
        {
            let metadata = sftp
                .metadata(&entry.remote_path)
                .await
                .inspect_err(|error| {
                    log::warn!(
                        "SFTP sync metadata-error: remote={}, error={error:#}",
                        entry.remote_path
                    )
                })
                .with_context(|| format!("读取远程文件 {} 信息失败", entry.remote_path))?;
            if metadata.is_dir() {
                bail!(
                    "远程目标类型冲突：本地文件 {} 对应远程目录 {}",
                    entry.local_path.display(),
                    entry.remote_path
                );
            }
            let remote_signature = FileSignature::from_remote(&metadata);
            if remote_signature.modified_at.is_none() {
                log::warn!(
                    "SFTP sync metadata-error: remote mtime unavailable, remote={}",
                    entry.remote_path
                );
            }
            if entry.signature.compare(&remote_signature) == SyncDecision::Unchanged {
                outcome.skipped_files = outcome.skipped_files.saturating_add(1);
                outcome.unchanged_entries = outcome.unchanged_entries.saturating_add(1);
                log::debug!(
                    "SFTP 上传跳过未修改文件: local={}, remote={}, size={}, modified_at={:?}",
                    entry.local_path.display(),
                    entry.remote_path,
                    entry.signature.size,
                    entry.signature.modified_at
                );
                continue;
            }
        }

        let mut source = tokio::fs::File::open(&entry.local_path)
            .await
            .with_context(|| format!("打开本地文件 {} 失败", entry.local_path.display()))?;
        let mut target = sftp
            .create(&entry.remote_path)
            .await
            .with_context(|| format!("创建远程文件 {} 失败", entry.remote_path))?;
        copy_with_progress(
            &mut source,
            &mut target,
            total_size,
            &mut transferred,
            &mut on_progress,
            &is_cancelled,
        )
        .await?;
        target
            .flush()
            .await
            .with_context(|| format!("刷新远程文件 {} 失败", entry.remote_path))?;
        if let Some(seconds) = entry.signature.modified_at {
            if let Ok(seconds) = u32::try_from(seconds) {
                let mut attributes = FileAttributes::empty();
                attributes.mtime = Some(seconds);
                if let Err(error) = target.set_metadata(attributes).await {
                    log::warn!(
                        "SFTP 上传后写入远程 mtime 失败: remote={}, mtime={}, error={error:#}",
                        entry.remote_path,
                        seconds
                    );
                }
            } else {
                log::warn!(
                    "SFTP 本地 mtime 超出远端支持范围: local={}, mtime={seconds}",
                    entry.local_path.display()
                );
            }
        }
        outcome.transferred_files = outcome.transferred_files.saturating_add(1);
        outcome.transferred_bytes = outcome
            .transferred_bytes
            .saturating_add(entry.signature.size);
    }
    on_progress(total_size, total_size);
    Ok(outcome)
}

pub(super) async fn download_path(
    sftp: &SftpSession,
    remote_path: &str,
    local_path: &Path,
    _known_size: u64,
    is_directory: bool,
    mut on_progress: impl FnMut(u64, u64),
    is_cancelled: impl Fn() -> bool,
) -> Result<TransferOutcome> {
    let (entries, total_size) =
        collect_remote_entries(sftp, remote_path, local_path, is_directory).await?;
    let mut transferred = 0_u64;
    let mut outcome = TransferOutcome {
        is_directory,
        total_size,
        transferred_files: 0,
        skipped_files: 0,
        unchanged_entries: 0,
        transferred_bytes: 0,
    };

    for entry in entries {
        if is_cancelled() {
            bail!("传输已取消");
        }
        if entry.is_directory {
            match tokio::fs::symlink_metadata(&entry.local_path).await {
                Ok(metadata) if metadata.is_dir() => {
                    outcome.unchanged_entries = outcome.unchanged_entries.saturating_add(1);
                }
                Ok(_) => {
                    bail!(
                        "本地目标类型冲突：远程目录 {} 对应本地文件 {}",
                        entry.remote_path,
                        entry.local_path.display()
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    tokio::fs::create_dir_all(&entry.local_path)
                        .await
                        .with_context(|| {
                            format!("创建本地目录 {} 失败", entry.local_path.display())
                        })?;
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("读取本地目录 {} 信息失败", entry.local_path.display())
                    });
                }
            }
            continue;
        }

        match tokio::fs::symlink_metadata(&entry.local_path).await {
            Ok(metadata) => {
                if metadata.is_dir() {
                    bail!(
                        "本地目标类型冲突：远程文件 {} 对应本地目录 {}",
                        entry.remote_path,
                        entry.local_path.display()
                    );
                }
                let local_signature = FileSignature::from_local(&metadata);
                if local_signature.compare(&entry.signature) == SyncDecision::Unchanged {
                    outcome.skipped_files = outcome.skipped_files.saturating_add(1);
                    outcome.unchanged_entries = outcome.unchanged_entries.saturating_add(1);
                    log::debug!(
                        "SFTP 下载跳过未修改文件: remote={}, local={}, size={}, modified_at={:?}",
                        entry.remote_path,
                        entry.local_path.display(),
                        entry.signature.size,
                        entry.signature.modified_at
                    );
                    continue;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                log::warn!(
                    "SFTP sync metadata-error: local={}, error={error}",
                    entry.local_path.display()
                );
                return Err(error).with_context(|| {
                    format!("读取本地文件 {} 信息失败", entry.local_path.display())
                });
            }
        }

        if let Some(parent) = entry.local_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("创建本地目录 {} 失败", parent.display()))?;
        }
        let mut source = sftp
            .open(&entry.remote_path)
            .await
            .with_context(|| format!("打开远程文件 {} 失败", entry.remote_path))?;
        let mut target = tokio::fs::File::create(&entry.local_path)
            .await
            .with_context(|| format!("创建本地文件 {} 失败", entry.local_path.display()))?;
        copy_with_progress(
            &mut source,
            &mut target,
            total_size,
            &mut transferred,
            &mut on_progress,
            &is_cancelled,
        )
        .await?;
        target
            .flush()
            .await
            .with_context(|| format!("刷新本地文件 {} 失败", entry.local_path.display()))?;
        if let Some(seconds) = entry.signature.modified_at {
            let local_path = entry.local_path.clone();
            tokio::task::spawn_blocking(move || {
                set_file_mtime(
                    &local_path,
                    FileTime::from_unix_time(
                        i64::from(u32::try_from(seconds).unwrap_or(u32::MAX)),
                        0,
                    ),
                )
                .with_context(|| format!("写入本地文件 {} mtime 失败", local_path.display()))
            })
            .await
            .context("写入本地文件 mtime 任务失败")??;
        }
        outcome.transferred_files = outcome.transferred_files.saturating_add(1);
        outcome.transferred_bytes = outcome
            .transferred_bytes
            .saturating_add(entry.signature.size);
    }
    on_progress(total_size, total_size);
    Ok(outcome)
}

fn collect_local_entries(
    local_root: &Path,
    remote_root: &str,
) -> Result<(Vec<LocalTransferEntry>, u64)> {
    let mut entries = Vec::new();
    let mut total_size = 0_u64;
    let mut pending = vec![(local_root.to_owned(), remote_root.to_owned())];
    while let Some((local_path, remote_path)) = pending.pop() {
        let metadata = std::fs::symlink_metadata(&local_path)
            .with_context(|| format!("读取本地路径 {} 失败", local_path.display()))?;
        if metadata.file_type().is_symlink() {
            bail!("暂不支持传输符号链接 {}", local_path.display());
        }
        let is_directory = metadata.is_dir();
        entries.push(LocalTransferEntry {
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            is_directory,
            signature: FileSignature::from_local(&metadata),
        });
        if !is_directory {
            total_size = total_size.saturating_add(metadata.len());
            continue;
        }
        for child in std::fs::read_dir(&local_path)
            .with_context(|| format!("读取本地目录 {} 失败", local_path.display()))?
        {
            let child =
                child.with_context(|| format!("读取本地目录项 {} 失败", local_path.display()))?;
            let name = child.file_name().to_string_lossy().into_owned();
            pending.push((child.path(), join_remote_path(&remote_path, &name)));
        }
    }
    Ok((entries, total_size))
}

async fn collect_remote_entries(
    sftp: &SftpSession,
    remote_root: &str,
    local_root: &Path,
    is_directory: bool,
) -> Result<(Vec<RemoteTransferEntry>, u64)> {
    if !is_directory {
        let metadata = sftp
            .metadata(remote_root)
            .await
            .inspect_err(|error| {
                log::warn!(
                    "SFTP sync metadata-error: remote={}, error={error:#}",
                    remote_root
                )
            })
            .with_context(|| format!("读取远程文件 {remote_root} 信息失败"))?;
        let total_size = metadata.len();
        return Ok((
            vec![RemoteTransferEntry {
                remote_path: remote_root.to_owned(),
                local_path: local_root.to_owned(),
                is_directory: false,
                signature: FileSignature::from_remote(&metadata),
            }],
            total_size,
        ));
    }

    let mut entries = Vec::new();
    let mut total_size = 0_u64;
    let mut pending = vec![(remote_root.to_owned(), local_root.to_owned())];
    while let Some((remote_path, local_path)) = pending.pop() {
        entries.push(RemoteTransferEntry {
            remote_path: remote_path.clone(),
            local_path: local_path.clone(),
            is_directory: true,
            signature: FileSignature::from_remote(
                &sftp
                    .metadata(&remote_path)
                    .await
                    .inspect_err(|error| {
                        log::warn!(
                            "SFTP sync metadata-error: remote={}, error={error:#}",
                            remote_path
                        )
                    })
                    .with_context(|| format!("读取远程目录 {remote_path} 信息失败"))?,
            ),
        });
        let children = sftp
            .read_dir(&remote_path)
            .await
            .with_context(|| format!("读取远程目录 {remote_path} 失败"))?;
        for child in children {
            let name = child.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let metadata = child.metadata();
            let child_local_path = local_path.join(&name);
            if metadata.is_dir() {
                pending.push((child.path(), child_local_path));
            } else {
                total_size = total_size.saturating_add(metadata.len());
                if metadata.mtime.is_none() {
                    log::warn!(
                        "SFTP sync metadata-error: remote mtime unavailable, remote={}",
                        child.path()
                    );
                }
                entries.push(RemoteTransferEntry {
                    remote_path: child.path(),
                    local_path: child_local_path,
                    is_directory: false,
                    signature: FileSignature::from_remote(&metadata),
                });
            }
        }
    }
    Ok((entries, total_size))
}
