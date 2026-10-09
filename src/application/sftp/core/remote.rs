use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Context as _, Result};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinSet;

use crate::{
    application::ports::SftpConnection,
    domain::{session::SessionProfile, terminal::TerminalStatus},
    infrastructure::INFRASTRUCTURE,
};

use super::{SftpCommand, SftpModel};

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
    let sftp: Arc<dyn SftpConnection> =
        Arc::from(INFRASTRUCTURE.connect_sftp(&workspace_id, &profile).await?);

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
    let entries = sftp.read_directory(&initial_path).await?;
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
                    SftpCommand::ChangeRemoteDirectory { path, complete } => {
                        log::debug!("SFTP 远程目录扫描开始: {path}");
                        let result = async {
                            let path = sftp.canonicalize(&path).await.context("解析远程目录失败")?;
                            let entries = sftp.read_directory(&path).await?;
                            Ok::<_, anyhow::Error>((path, entries))
                        }
                        .await;
                        match result {
                            Ok((path, entries)) => {
                                log::debug!(
                                    "SFTP 远程目录扫描完成: path={path}, entries={}",
                                    entries.len()
                                );
                                model.set_directory(path.clone(), entries);
                                let _ = complete.send(Ok(path));
                            }
                            Err(error) => {
                                log::warn!("SFTP 远程目录扫描失败: {error:#}");
                                model.set_error(format!("{error:#}"));
                                let _ = complete.send(Err(format!("{error:#}")));
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
                        let refresh_generation = model.remote_refresh_generation(&refresh_path);
                        let delete_result = sftp.delete_paths(&items).await;
                        if let Err(error) = &delete_result {
                            log::warn!("SFTP 远程删除失败: workspace_id={workspace_id}, error={error:#}");
                        }
                        if let Some(generation) = refresh_generation {
                            match sftp.read_directory(&refresh_path).await {
                                Ok(entries) => {
                                    if model.set_directory_if_current(
                                        refresh_path.clone(), generation, entries,
                                    ) {
                                        if let Err(error) = delete_result {
                                            model.set_error_if_current(
                                                &refresh_path,
                                                generation,
                                                format!("{error:#}"),
                                            );
                                        }
                                    } else {
                                        log::debug!(
                                            "SFTP delete directory refresh skipped after navigation: workspace_id={workspace_id}, path={refresh_path}"
                                        );
                                    }
                                }
                                Err(error) => model.set_error_if_current(
                                    &refresh_path,
                                    generation,
                                    format!("{error:#}"),
                                ),
                            }
                        } else {
                            log::debug!(
                                "SFTP delete directory refresh skipped for stale path: workspace_id={workspace_id}, path={refresh_path}"
                            );
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

    if let Err(error) = sftp.close().await {
        log::debug!(
            "SFTP protocol adapter close failed: workspace_id={workspace_id}, error={error:#}"
        );
    }
    model.update(|snapshot| {
        snapshot.status = TerminalStatus::Disconnected;
        snapshot.loading = false;
    });
    log::debug!("SFTP runtime stopped: workspace_id={workspace_id}");
    Ok(())
}

async fn run_upload(
    sftp: Arc<dyn SftpConnection>,
    model: Arc<SftpModel>,
    transfer_id: u64,
    local_path: PathBuf,
    remote_path: String,
    refresh_path: String,
    target_lock: Arc<Mutex<()>>,
    complete: Option<oneshot::Sender<bool>>,
) {
    let _target_lock = target_lock.lock().await;
    let progress_model = model.clone();
    let cancel_model = model.clone();
    let result = sftp
        .upload_path(
            &local_path,
            &remote_path,
            Box::new(move |transferred, total| {
                let progress = if total == 0 {
                    1.
                } else {
                    transferred as f32 / total as f32
                };
                progress_model.update_transfer(transfer_id, progress, transferred, total, "传输中")
            }),
            Box::new(move || cancel_model.is_cancelled(transfer_id)),
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
                if let Some(generation) = model.remote_refresh_generation(&refresh_path) {
                    match sftp.read_directory(&refresh_path).await {
                        Ok(entries) => {
                            if !model.set_directory_if_current(refresh_path, generation, entries) {
                                log::debug!(
                                    "SFTP upload directory refresh skipped after navigation: transfer_id={transfer_id}"
                                );
                            }
                        }
                        Err(error) => {
                            model.set_error_if_current(
                                &refresh_path,
                                generation,
                                format!("{error:#}"),
                            );
                        }
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
    sftp: Arc<dyn SftpConnection>,
    model: Arc<SftpModel>,
    transfer_id: u64,
    remote_path: String,
    local_path: PathBuf,
    _total_size: u64,
    is_directory: bool,
    target_lock: Arc<Mutex<()>>,
    complete: oneshot::Sender<bool>,
) {
    let _target_lock = target_lock.lock().await;
    let progress_model = model.clone();
    let cancel_model = model.clone();
    let result = sftp
        .download_path(
            &remote_path,
            &local_path,
            is_directory,
            Box::new(move |transferred, total| {
                let progress = if total == 0 {
                    1.
                } else {
                    transferred as f32 / total as f32
                };
                progress_model.update_transfer(transfer_id, progress, transferred, total, "传输中")
            }),
            Box::new(move || cancel_model.is_cancelled(transfer_id)),
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

pub(super) fn join_remote_path(directory: &str, file_name: &str) -> String {
    if directory == "/" {
        format!("/{file_name}")
    } else {
        format!("{}/{file_name}", directory.trim_end_matches('/'))
    }
}
