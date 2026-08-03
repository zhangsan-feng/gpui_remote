use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context as _, Result, bail};
use russh::{Disconnect, client};
use russh_sftp::client::SftpSession;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc,
};

use crate::domain::session::SessionProfile;

use super::super::{SftpCommand, SftpEntry, SftpModel, SftpStatus};
use super::conn::{SftpClientHandler, connect_transport, ssh_config};

const TRANSFER_BUFFER_SIZE: usize = 64 * 1024;

pub(super) async fn run_sftp(
    profile: SessionProfile,
    mut commands: mpsc::UnboundedReceiver<SftpCommand>,
    model: Arc<SftpModel>,
) -> Result<()> {
    let stream = connect_transport(&profile).await?;
    let config = Arc::new(ssh_config());
    let mut session = client::connect_stream(
        config,
        stream,
        SftpClientHandler {
            endpoint: format!("[{}]:{}", profile.host, profile.port),
        },
    )
    .await
    .context("SFTP SSH 握手或主机密钥校验失败")?;
    let authentication = session
        .authenticate_password(profile.username.clone(), profile.password.clone())
        .await
        .context("SFTP SSH 密码认证失败")?;
    if !authentication.success() {
        bail!("SFTP 用户名或密码错误");
    }

    let channel = session
        .channel_open_session()
        .await
        .context("创建 SFTP 会话通道失败")?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .context("启动远程 SFTP 子系统失败")?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .context("初始化 SFTP 协议失败")?;

    let initial_path = sftp
        .canonicalize(".")
        .await
        .context("读取 SFTP 初始目录失败")?;
    let entries = read_directory(&sftp, &initial_path).await?;
    model.set_connected(initial_path, entries);

    while let Some(command) = commands.recv().await {
        match command {
            SftpCommand::LoadDirectory(path) => {
                let result = async {
                    let path = sftp.canonicalize(path).await.context("解析远程目录失败")?;
                    let entries = read_directory(&sftp, &path).await?;
                    Ok::<_, anyhow::Error>((path, entries))
                }
                .await;
                match result {
                    Ok((path, entries)) => model.set_directory(path, entries),
                    Err(error) => model.set_error(format!("{error:#}")),
                }
            }
            SftpCommand::Upload {
                transfer_id,
                local_path,
                remote_path,
                refresh_path,
            } => {
                model.update_transfer(transfer_id, 0., "扫描中");
                let result = upload_path(&sftp, &local_path, &remote_path, |progress| {
                    model.update_transfer(transfer_id, progress, "传输中")
                })
                .await;
                match result {
                    Ok(()) => {
                        model.update_transfer(transfer_id, 1., "已完成");
                        if model.snapshot().path == refresh_path {
                            match read_directory(&sftp, &refresh_path).await {
                                Ok(entries) => model.set_directory(refresh_path, entries),
                                Err(error) => model.set_error(format!("{error:#}")),
                            }
                        }
                    }
                    Err(error) => {
                        log::error!("上传文件失败: {error:#}");
                        model.update_transfer(transfer_id, 0., "失败");
                    }
                }
            }
            SftpCommand::Download {
                transfer_id,
                remote_path,
                local_path,
                total_size,
                is_directory,
                complete,
            } => {
                model.update_transfer(transfer_id, 0., "扫描中");
                let result = download_path(
                    &sftp,
                    &remote_path,
                    &local_path,
                    total_size,
                    is_directory,
                    |progress| model.update_transfer(transfer_id, progress, "传输中"),
                )
                .await;
                let succeeded = result.is_ok();
                if let Err(error) = result {
                    log::error!("下载文件失败: {error:#}");
                    model.update_transfer(transfer_id, 0., "失败");
                } else {
                    model.update_transfer(transfer_id, 1., "已完成");
                }
                let _ = complete.send(succeeded);
            }
            SftpCommand::Delete {
                path,
                is_directory,
                refresh_path,
            } => match delete_remote_path(&sftp, &path, is_directory).await {
                Ok(()) => match read_directory(&sftp, &refresh_path).await {
                    Ok(entries) => model.set_directory(refresh_path, entries),
                    Err(error) => model.set_error(format!("{error:#}")),
                },
                Err(error) => model.set_error(format!("{error:#}")),
            },
            SftpCommand::Disconnect => break,
        }
    }

    let _ = sftp.close().await;
    let _ = session
        .disconnect(Disconnect::ByApplication, "", "zh-CN")
        .await;
    model.update(
        |snapshot| {
            snapshot.status = SftpStatus::Disconnected;
            snapshot.loading = false;
        },
        true,
    );
    Ok(())
}

struct LocalTransferEntry {
    local_path: PathBuf,
    remote_path: String,
    is_directory: bool,
}

struct RemoteTransferEntry {
    remote_path: String,
    local_path: PathBuf,
    is_directory: bool,
}

async fn upload_path(
    sftp: &SftpSession,
    local_path: &Path,
    remote_path: &str,
    mut on_progress: impl FnMut(f32),
) -> Result<()> {
    let local_root = local_path.to_owned();
    let remote_root = remote_path.to_owned();
    let (entries, total_size) =
        tokio::task::spawn_blocking(move || collect_local_entries(&local_root, &remote_root))
            .await
            .context("扫描本地上传目录任务失败")??;
    let mut transferred = 0_u64;

    for entry in entries {
        if entry.is_directory {
            if !sftp
                .try_exists(&entry.remote_path)
                .await
                .with_context(|| format!("检查远程目录 {} 失败", entry.remote_path))?
            {
                sftp.create_dir(&entry.remote_path)
                    .await
                    .with_context(|| format!("创建远程目录 {} 失败", entry.remote_path))?;
            }
            continue;
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
        )
        .await?;
        target
            .flush()
            .await
            .with_context(|| format!("刷新远程文件 {} 失败", entry.remote_path))?;
    }
    on_progress(1.);
    Ok(())
}

async fn download_path(
    sftp: &SftpSession,
    remote_path: &str,
    local_path: &Path,
    known_size: u64,
    is_directory: bool,
    mut on_progress: impl FnMut(f32),
) -> Result<()> {
    let (entries, total_size) =
        collect_remote_entries(sftp, remote_path, local_path, known_size, is_directory).await?;
    let mut transferred = 0_u64;
    for entry in entries {
        if entry.is_directory {
            tokio::fs::create_dir_all(&entry.local_path)
                .await
                .with_context(|| format!("创建本地目录 {} 失败", entry.local_path.display()))?;
            continue;
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
        )
        .await?;
        target
            .flush()
            .await
            .with_context(|| format!("刷新本地文件 {} 失败", entry.local_path.display()))?;
    }
    on_progress(1.);
    Ok(())
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
    known_size: u64,
    is_directory: bool,
) -> Result<(Vec<RemoteTransferEntry>, u64)> {
    if !is_directory {
        let total_size = if known_size == 0 {
            sftp.metadata(remote_root)
                .await
                .with_context(|| format!("读取远程文件 {remote_root} 信息失败"))?
                .len()
        } else {
            known_size
        };
        return Ok((
            vec![RemoteTransferEntry {
                remote_path: remote_root.to_owned(),
                local_path: local_root.to_owned(),
                is_directory: false,
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
                entries.push(RemoteTransferEntry {
                    remote_path: child.path(),
                    local_path: child_local_path,
                    is_directory: false,
                });
            }
        }
    }
    Ok((entries, total_size))
}

async fn delete_remote_path(
    sftp: &SftpSession,
    remote_path: &str,
    is_directory: bool,
) -> Result<()> {
    if !is_directory {
        return sftp
            .remove_file(remote_path)
            .await
            .with_context(|| format!("删除远程文件 {remote_path} 失败"));
    }
    let mut directories = Vec::new();
    let mut pending = vec![remote_path.to_owned()];
    while let Some(directory) = pending.pop() {
        directories.push(directory.clone());
        let children = sftp
            .read_dir(&directory)
            .await
            .with_context(|| format!("读取远程目录 {directory} 失败"))?;
        for child in children {
            let name = child.file_name();
            if name == "." || name == ".." {
                continue;
            }
            if child.metadata().is_dir() {
                pending.push(child.path());
            } else {
                let child_path = child.path();
                sftp.remove_file(&child_path)
                    .await
                    .with_context(|| format!("删除远程文件 {child_path} 失败"))?;
            }
        }
    }
    for directory in directories.into_iter().rev() {
        sftp.remove_dir(&directory)
            .await
            .with_context(|| format!("删除远程目录 {directory} 失败"))?;
    }
    Ok(())
}

async fn copy_with_progress(
    source: &mut (impl AsyncRead + Unpin),
    target: &mut (impl AsyncWrite + Unpin),
    total_size: u64,
    transferred: &mut u64,
    on_progress: &mut impl FnMut(f32),
) -> Result<()> {
    let mut buffer = vec![0; TRANSFER_BUFFER_SIZE];
    loop {
        let read = source.read(&mut buffer).await.context("读取传输数据失败")?;
        if read == 0 {
            break;
        }
        target
            .write_all(&buffer[..read])
            .await
            .context("写入传输数据失败")?;
        *transferred = transferred.saturating_add(read as u64);
        let progress = if total_size == 0 {
            1.
        } else {
            *transferred as f32 / total_size as f32
        };
        on_progress(progress);
    }
    Ok(())
}

pub(super) fn join_remote_path(directory: &str, file_name: &str) -> String {
    if directory == "/" {
        format!("/{file_name}")
    } else {
        format!("{}/{file_name}", directory.trim_end_matches('/'))
    }
}

async fn read_directory(sftp: &SftpSession, path: &str) -> Result<Vec<SftpEntry>> {
    let mut entries = sftp
        .read_dir(path)
        .await
        .with_context(|| format!("读取远程目录 {path} 失败"))?
        .map(|entry| {
            let metadata = entry.metadata();
            SftpEntry {
                name: entry.file_name(),
                path: entry.path(),
                is_directory: metadata.is_dir(),
                size: metadata.len(),
                modified_at: metadata.mtime,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}
