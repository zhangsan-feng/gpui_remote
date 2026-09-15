use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result, bail};
use russh::{Disconnect, client};
use russh_sftp::client::SftpSession;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinSet;

use crate::{
    domain::session::SessionProfile,
    infrastructure::proxy::{ProxySettings, connect},
};

use super::conn::{SftpClientHandler, ssh_config};
use super::transfer::{download_path, upload_path};
use super::{RemoteDeleteItem, SftpCommand, SftpEntry, SftpModel, SftpStatus};

pub(super) async fn run_sftp_session(
    workspace_id: String,
    profile: SessionProfile,
    initial_remote_path: Option<String>,
    mut commands: mpsc::UnboundedReceiver<SftpCommand>,
    model: Arc<SftpModel>,
) -> Result<()> {
    log::debug!(
        "SFTP runtime starting: workspace_id={workspace_id}, profile_id={}, host={}, port={}",
        profile.id,
        profile.host,
        profile.port
    );
    let proxy = profile.proxy.as_ref().map(|proxy| ProxySettings {
        host: proxy.host.clone(),
        port: proxy.port,
        username: proxy.username.clone(),
        password: proxy.password.clone(),
    });
    let stream = connect((profile.host.as_str(), profile.port), proxy.as_ref()).await?;
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
    let authentication = if let Some(path) = profile.private_key_path.as_deref() {
        let key_path = path.to_owned();
        let key = tokio::task::spawn_blocking({
            let key_path = key_path.clone();
            move || russh::keys::load_secret_key(&key_path, None)
        })
        .await
        .context("加载 SFTP SSH 私钥任务失败")?
        .with_context(|| format!("加载 SFTP SSH 私钥失败: {key_path}"))?;
        session
            .authenticate_publickey(
                profile.username.clone(),
                russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
            )
            .await
            .context("SFTP SSH 私钥认证失败")?
    } else {
        session
            .authenticate_password(profile.username.clone(), profile.password.clone())
            .await
            .context("SFTP SSH 密码认证失败")?
    };
    if !authentication.success() {
        if profile.private_key_path.is_some() {
            bail!("SFTP 用户名或私钥错误");
        }
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
    let sftp = Arc::new(
        SftpSession::new(channel.into_stream())
            .await
            .context("初始化 SFTP 协议失败")?,
    );

    let initial_path = if let Some(path) = initial_remote_path.as_deref() {
        match sftp.canonicalize(path).await {
            Ok(path) => path,
            Err(error) => {
                log::warn!("恢复 SFTP 远程目录失败（{path}）：{error:#}，将使用默认目录");
                sftp.canonicalize(".")
                    .await
                    .context("读取 SFTP 初始目录失败")?
            }
        }
    } else {
        sftp.canonicalize(".")
            .await
            .context("读取 SFTP 初始目录失败")?
    };
    log::debug!("SFTP 初始远程目录扫描开始: workspace_id={workspace_id}, path={initial_path}");
    let entries = scan_remote_directory(&sftp, &initial_path).await?;
    log::debug!(
        "SFTP 初始远程目录扫描完成: workspace_id={workspace_id}, path={}, entries={}",
        initial_path,
        entries.len()
    );
    model.set_connected(initial_path, entries);

    let mut transfer_tasks = JoinSet::new();
    let mut transfer_locks = HashMap::<String, Arc<Mutex<()>>>::new();
    loop {
        tokio::select! {
            command = commands.recv() => {
                let command = match command {
                    Some(command) => command,
                    None => {
                        transfer_tasks.abort_all();
                        while let Some(result) = transfer_tasks.join_next().await {
                            if let Err(error) = result {
                                log::debug!("SFTP 传输任务结束: {error}");
                            }
                        }
                        break;
                    }
                };
                match command {
                    SftpCommand::ChangeRemoteDirectory(path) => {
                        log::debug!("SFTP 远程目录扫描开始: {path}");
                        let result = async {
                            let path = sftp.canonicalize(path).await.context("解析远程目录失败")?;
                            let entries = scan_remote_directory(&sftp, &path).await?;
                            Ok::<_, anyhow::Error>((path, entries))
                        }
                        .await;
                        match result {
                            Ok((path, entries)) => {
                                log::debug!(
                                    "SFTP 远程目录扫描完成: path={path}, entries={}",
                                    entries.len()
                                );
                                model.set_directory(path, entries)
                            }
                            Err(error) => {
                                log::warn!("SFTP 远程目录扫描失败: {error:#}");
                                model.set_error(format!("{error:#}"));
                            }
                        }
                    }
                    SftpCommand::Upload {
                        transfer_id,
                        local_path,
                        remote_path,
                        refresh_path,
                        complete,
                    } => {
                        if model.is_cancelled(transfer_id) {
                            model.update_transfer(transfer_id, 0., 0, 0, "已取消");
                            if let Some(complete) = complete {
                                let _ = complete.send(false);
                            }
                            continue;
                        }
                        model.update_transfer(transfer_id, 0., 0, 0, "扫描中");
                        log::debug!(
                            "SFTP 上传任务开始: transfer_id={transfer_id}, local={}, remote={remote_path}",
                            local_path.display()
                        );
                        let target_lock = transfer_locks
                            .entry(remote_path.clone())
                            .or_insert_with(|| Arc::new(Mutex::new(())))
                            .clone();
                        transfer_tasks.spawn(run_upload(
                            sftp.clone(),
                            model.clone(),
                            transfer_id,
                            local_path,
                            remote_path,
                            refresh_path,
                            target_lock,
                            complete,
                        ));
                    }
                    SftpCommand::Download {
                        transfer_id,
                        remote_path,
                        local_path,
                        total_size,
                        is_directory,
                        complete,
                    } => {
                        if model.is_cancelled(transfer_id) {
                            model.update_transfer(transfer_id, 0., 0, 0, "已取消");
                            let _ = complete.send(false);
                            continue;
                        }
                        model.update_transfer(transfer_id, 0., 0, 0, "扫描中");
                        log::debug!(
                            "SFTP 下载任务开始: transfer_id={transfer_id}, remote={remote_path}, local={}",
                            local_path.display()
                        );
                        let target_lock = transfer_locks
                            .entry(remote_path.clone())
                            .or_insert_with(|| Arc::new(Mutex::new(())))
                            .clone();
                        transfer_tasks.spawn(run_download(
                            sftp.clone(),
                            model.clone(),
                            transfer_id,
                            remote_path,
                            local_path,
                            total_size,
                            is_directory,
                            target_lock,
                            complete,
                        ));
                    }
                    SftpCommand::Delete {
                        items,
                        refresh_path,
                    } => {
                        let delete_result = delete_remote_paths(&sftp, &items).await;
                        match scan_remote_directory(&sftp, &refresh_path).await {
                            Ok(entries) => {
                                model.set_directory(refresh_path, entries);
                                if let Err(error) = delete_result {
                                    model.set_error(format!("{error:#}"));
                                }
                            }
                            Err(error) => model.set_error(format!("{error:#}")),
                        }
                    }
                    SftpCommand::Disconnect => {
                        transfer_tasks.abort_all();
                        while let Some(result) = transfer_tasks.join_next().await {
                            if let Err(error) = result {
                                log::debug!("SFTP 传输任务结束: {error}");
                            }
                        }
                        break;
                    }
                }
            }
            result = transfer_tasks.join_next(), if !transfer_tasks.is_empty() => {
                if let Some(Err(error)) = result {
                    log::warn!("SFTP 传输任务异常结束: {error}");
                }
            }
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
    log::debug!("SFTP runtime stopped: workspace_id={workspace_id}");
    Ok(())
}

async fn run_upload(
    sftp: Arc<SftpSession>,
    model: Arc<SftpModel>,
    transfer_id: u64,
    local_path: PathBuf,
    remote_path: String,
    refresh_path: String,
    target_lock: Arc<Mutex<()>>,
    complete: Option<oneshot::Sender<bool>>,
) {
    let _target_lock = target_lock.lock().await;
    let result = upload_path(
        &sftp,
        &local_path,
        &remote_path,
        |transferred, total| {
            let progress = if total == 0 {
                1.
            } else {
                transferred as f32 / total as f32
            };
            model.update_transfer(transfer_id, progress, transferred, total, "传输中")
        },
        || model.is_cancelled(transfer_id),
    )
    .await;
    match result {
        Ok(outcome) => {
            let succeeded = !model.is_cancelled(transfer_id);
            model.set_upload_directory(transfer_id, outcome.is_directory);
            if model.is_cancelled(transfer_id) {
                model.update_transfer(transfer_id, 0., 0, 0, "已取消");
            } else {
                let status = if outcome.transferred_files == 0 && outcome.unchanged_entries > 0 {
                    "未修改"
                } else {
                    "已完成"
                };
                model.update_transfer(
                    transfer_id,
                    1.,
                    outcome.transferred_bytes,
                    outcome.total_size,
                    status,
                );
                log::debug!(
                    "SFTP 上传结果: transfer_id={transfer_id}, transferred_files={}, skipped_files={}, unchanged_entries={}, transferred_bytes={}",
                    outcome.transferred_files,
                    outcome.skipped_files,
                    outcome.unchanged_entries,
                    outcome.transferred_bytes
                );
                if model.snapshot().path == refresh_path {
                    match scan_remote_directory(&sftp, &refresh_path).await {
                        Ok(entries) => model.set_directory(refresh_path, entries),
                        Err(error) => model.set_error(format!("{error:#}")),
                    }
                }
            }
            if let Some(complete) = complete {
                let _ = complete.send(succeeded);
            }
        }
        Err(error) => {
            if model.is_cancelled(transfer_id) {
                model.update_transfer(transfer_id, 0., 0, 0, "已取消");
            } else {
                log::error!("上传文件失败: {error:#}");
                model.set_transfer_error(transfer_id, format!("{error:#}"));
            }
            if let Some(complete) = complete {
                let _ = complete.send(false);
            }
        }
    }
    log::debug!("SFTP 上传任务结束: transfer_id={transfer_id}");
}

async fn run_download(
    sftp: Arc<SftpSession>,
    model: Arc<SftpModel>,
    transfer_id: u64,
    remote_path: String,
    local_path: PathBuf,
    total_size: u64,
    is_directory: bool,
    target_lock: Arc<Mutex<()>>,
    complete: oneshot::Sender<bool>,
) {
    let _target_lock = target_lock.lock().await;
    let result = download_path(
        &sftp,
        &remote_path,
        &local_path,
        total_size,
        is_directory,
        |transferred, total| {
            let progress = if total == 0 {
                1.
            } else {
                transferred as f32 / total as f32
            };
            model.update_transfer(transfer_id, progress, transferred, total, "传输中")
        },
        || model.is_cancelled(transfer_id),
    )
    .await;
    let mut refresh_local_directory = false;
    if model.is_cancelled(transfer_id) {
        model.update_transfer(transfer_id, 0., 0, 0, "已取消");
    } else {
        match result {
            Ok(outcome) => {
                let status = if outcome.transferred_files == 0 && outcome.unchanged_entries > 0 {
                    "未修改"
                } else {
                    "已完成"
                };
                model.update_transfer(
                    transfer_id,
                    1.,
                    outcome.transferred_bytes,
                    outcome.total_size,
                    status,
                );
                refresh_local_directory = outcome.transferred_files > 0;
                log::debug!(
                    "SFTP 下载结果: transfer_id={transfer_id}, transferred_files={}, skipped_files={}, unchanged_entries={}, transferred_bytes={}",
                    outcome.transferred_files,
                    outcome.skipped_files,
                    outcome.unchanged_entries,
                    outcome.transferred_bytes
                );
            }
            Err(error) => {
                log::error!("下载文件失败: {error:#}");
                model.set_transfer_error(transfer_id, format!("{error:#}"));
            }
        }
    }
    let _ = complete.send(refresh_local_directory);
    log::debug!("SFTP 下载任务结束: transfer_id={transfer_id}");
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

async fn delete_remote_paths(sftp: &SftpSession, items: &[RemoteDeleteItem]) -> Result<()> {
    log::debug!("SFTP 批量删除远程路径开始: count={}", items.len());
    let mut errors = Vec::new();
    for item in items {
        log::debug!(
            "SFTP 删除远程路径: path={}, is_directory={}",
            item.path,
            item.is_directory
        );
        if let Err(error) = delete_remote_path(sftp, &item.path, item.is_directory).await {
            let error =
                anyhow::anyhow!(error).context(format!("批量删除远程路径 {} 失败", item.path));
            log::warn!("{error:#}");
            errors.push(format!("{error:#}"));
        }
    }
    if !errors.is_empty() {
        bail!("{}", errors.join("\n"));
    }
    log::debug!("SFTP 批量删除远程路径完成: count={}", items.len());
    Ok(())
}

pub(super) fn join_remote_path(directory: &str, file_name: &str) -> String {
    if directory == "/" {
        format!("/{file_name}")
    } else {
        format!("{}/{file_name}", directory.trim_end_matches('/'))
    }
}

async fn scan_remote_directory(sftp: &SftpSession, path: &str) -> Result<Vec<SftpEntry>> {
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
