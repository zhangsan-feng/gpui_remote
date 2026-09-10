mod conn;
mod delete;
mod local;
mod path;
mod remote;
mod watcher;

use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};

use gpui_kit::*;
use tokio::sync::{mpsc, oneshot};

use crate::{domain::session::SessionProfile, infrastructure::storage::Storage};

pub(super) use path::default_desktop_path;
pub(crate) use watcher::LocalWatch;

use super::{
    CancelTransfer, DownloadRemoteEntry, RetryTransfer, SftpCommand, SftpModel, SftpRuntime,
    SftpSnapshot, SftpStatus, SftpView, TransferRecord, TransferRequest, UploadLocalEntry,
};

impl SftpModel {
    pub(super) fn snapshot(&self) -> SftpSnapshot {
        self.snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn update(&self, update: impl FnOnce(&mut SftpSnapshot), status_changed: bool) {
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            update(&mut snapshot);
        }
        self.updates.notify_one();
        if status_changed {
            self.status_updates.notify_waiters();
        }
    }

    fn set_connected(&self, path: String, entries: Vec<super::SftpEntry>) {
        let path_changed = self.snapshot().path != path;
        self.update(
            move |snapshot| {
                snapshot.status = SftpStatus::Connected;
                snapshot.path = path;
                snapshot.entries = Arc::new(entries);
                snapshot.loading = false;
                snapshot.error = None;
            },
            true,
        );
        if path_changed {
            self.directory_updates.notify_one();
        }
    }

    fn set_loading(&self) {
        self.update(
            |snapshot| {
                snapshot.loading = true;
                snapshot.error = None;
            },
            false,
        );
    }

    fn set_directory(&self, path: String, entries: Vec<super::SftpEntry>) {
        let path_changed = self.snapshot().path != path;
        self.update(
            move |snapshot| {
                snapshot.path = path;
                snapshot.entries = Arc::new(entries);
                snapshot.loading = false;
                snapshot.error = None;
            },
            false,
        );
        if path_changed {
            self.directory_updates.notify_one();
        }
    }

    fn set_error(&self, error: String) {
        self.update(
            |snapshot| {
                snapshot.loading = false;
                snapshot.error = Some(error);
            },
            false,
        );
    }

    fn set_failed(&self, error: String) {
        self.update(
            |snapshot| {
                snapshot.status = SftpStatus::Failed;
                snapshot.loading = false;
                snapshot.error = Some(error);
            },
            true,
        );
    }

    fn update_transfer(
        &self,
        transfer_id: u64,
        progress: f32,
        transferred: u64,
        total: u64,
        status: impl Into<String>,
    ) {
        let status = status.into();
        let is_progress = status == "传输中" && transferred > 0;
        let now = std::time::Instant::now();
        let mut transfers = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(transfer) = transfers
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            transfer.progress = progress.clamp(0., 1.);
            if status == "扫描中" {
                transfer.speed = 0;
                transfer.transferred_bytes = 0;
                transfer.total_bytes = 0;
                transfer.error = None;
                transfer.started_at = None;
                transfer.speed_updated_at = None;
            } else if transferred > 0 {
                transfer.transferred_bytes = transferred;
                transfer.total_bytes = total;
                let started_at = transfer.started_at.get_or_insert(now);
                let elapsed = now.duration_since(*started_at).as_secs_f64();
                let should_update = transfer.speed_updated_at.is_none_or(|updated_at| {
                    now.duration_since(updated_at) >= Duration::from_secs(1)
                });
                if elapsed > 0. && should_update {
                    transfer.speed = (transferred as f64 / elapsed) as u64;
                    transfer.speed_updated_at = Some(now);
                }
            }
            transfer.status = status;
        }
        drop(transfers);

        let should_notify = if is_progress {
            let mut last_notify = self
                .transfer_ui_throttle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let should_notify = last_notify
                .is_none_or(|updated_at| now.duration_since(updated_at) >= Duration::from_secs(1));
            if should_notify {
                *last_notify = Some(now);
            }
            should_notify
        } else {
            true
        };
        if should_notify {
            self.updates.notify_one();
        }
    }

    pub(super) fn request_cancel(&self, transfer_id: u64) {
        self.cancelled_transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(transfer_id);
        self.set_transfer_status(transfer_id, "已取消");
    }

    pub(super) fn clear_cancel(&self, transfer_id: u64) {
        self.cancelled_transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&transfer_id);
    }

    pub(super) fn is_cancelled(&self, transfer_id: u64) -> bool {
        self.cancelled_transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(&transfer_id)
    }

    fn set_transfer_status(&self, transfer_id: u64, status: &str) {
        if let Some(transfer) = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            transfer.status = status.to_owned();
            if status == "已取消" {
                transfer.speed = 0;
            }
        }
        self.updates.notify_one();
    }

    pub(super) fn set_transfer_error(&self, transfer_id: u64, error: String) {
        if let Some(transfer) = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            transfer.status = "失败".to_owned();
            transfer.error = Some(error);
            transfer.speed = 0;
        }
        self.updates.notify_one();
    }

    fn set_upload_directory(&self, transfer_id: u64, is_directory: bool) {
        if let Some(transfer) = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            if let TransferRequest::Upload {
                is_directory: current,
                ..
            } = &mut transfer.request
            {
                *current = is_directory;
            }
        }
        self.updates.notify_one();
    }
}

impl SftpView {
    pub(super) fn load_local_directory(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let save_workspace_id = self.selected_workspace_id.clone();
        let should_persist_local_path = self.local.path != path;
        self.local_selection.clear();
        self.local.path = path.clone();
        self.local.loading = true;
        self.local.error = None;
        self.local_list_state.reset_with_uniform_height(0, px(38.));
        cx.notify();
        log::debug!("SFTP 本地目录扫描开始: {}", path.display());

        cx.spawn(async move |this, cx| {
            let result = tokio::task::spawn_blocking(move || local::read_local_directory(&path))
                .await
                .map_err(|error| anyhow::anyhow!("读取本地目录任务失败: {error}"))
                .and_then(|result| result);
            let _ = this.update(cx, |this, cx| {
                if this.selected_workspace_id != save_workspace_id {
                    return;
                }
                this.local.loading = false;
                match result {
                    Ok((path, entries)) => {
                        log::debug!(
                            "SFTP 本地目录扫描完成: {}, entries={}",
                            path.display(),
                            entries.len()
                        );
                        this.local.path = path.clone();
                        this.local.entries = Arc::new(entries);
                        this.local.error = None;
                        if should_persist_local_path {
                            if let Some(workspace_id) = save_workspace_id.as_deref() {
                                this.persist_local_directory(workspace_id, &path, cx);
                            }
                        }
                    }
                    Err(error) => {
                        log::warn!("SFTP 本地目录扫描失败: {error:#}");
                        this.local.error = Some(format!("{error:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn restore_local_directory(&mut self, workspace_id: &str, cx: &mut Context<Self>) {
        let Some(profile_id) = self
            .runtimes
            .get(workspace_id)
            .map(|runtime| runtime.profile_id.clone())
        else {
            return;
        };
        let session = cx.global::<Storage>().session.clone();
        let workspace_id = workspace_id.to_owned();
        cx.spawn(async move |this, cx| {
            let local_path = tokio::task::spawn_blocking(move || {
                let state = session.sftp_state(&profile_id)?;
                Ok::<_, anyhow::Error>(
                    state
                        .and_then(|state| state.local_path)
                        .unwrap_or_else(default_desktop_path),
                )
            })
            .await
            .map_err(|error| anyhow::anyhow!("读取 SFTP 本地目录任务失败: {error}"))
            .and_then(|result| result);
            let local_path = match local_path {
                Ok(path) => path,
                Err(error) => {
                    log::warn!("读取 SFTP 本地目录失败，会话 {workspace_id}: {error:#}");
                    tokio::task::spawn_blocking(default_desktop_path)
                        .await
                        .unwrap_or_else(|task_error| {
                            log::warn!("解析默认本地目录任务失败: {task_error}");
                            PathBuf::from(".")
                        })
                }
            };
            let _ = this.update(cx, |this, cx| {
                if this.selected_workspace_id.as_deref() != Some(workspace_id.as_str()) {
                    return;
                }
                this.load_local_directory(local_path, cx);
            });
        })
        .detach();
    }

    fn persist_local_directory(
        &self,
        workspace_id: &str,
        path: &std::path::Path,
        cx: &mut Context<Self>,
    ) {
        let Some(profile_id) = self
            .runtimes
            .get(workspace_id)
            .map(|runtime| runtime.profile_id.clone())
        else {
            return;
        };
        let session = cx.global::<Storage>().session.clone();
        let path = path.to_owned();
        let task_profile_id = profile_id.clone();
        log::debug!(
            "SFTP 本地目录路径变化，准备保存: session={}, path={}",
            profile_id,
            path.display()
        );
        cx.spawn(async move |_this, _cx| {
            let result = tokio::task::spawn_blocking(move || {
                session.update_sftp_local_path(&task_profile_id, &path)
            })
            .await;
            match result {
                Ok(Ok(())) => {
                    log::debug!("SFTP 本地目录保存完成: session={profile_id}");
                }
                Ok(Err(error)) => {
                    log::warn!("保存 SFTP 本地目录失败，会话 {profile_id}: {error:#}");
                }
                Err(error) => {
                    log::warn!("保存 SFTP 本地目录任务失败，会话 {profile_id}: {error}");
                }
            }
        })
        .detach();
    }

    pub(super) fn persist_remote_directories(&mut self, cx: &mut Context<Self>) {
        let directories = self
            .runtimes
            .iter()
            .filter_map(|(workspace_id, runtime)| {
                let path = runtime.model.snapshot().path;
                (!path.is_empty()).then(|| (workspace_id.clone(), runtime.profile_id.clone(), path))
            })
            .collect::<Vec<_>>();
        for (workspace_id, profile_id, path) in directories {
            if self
                .persisted_remote_paths
                .get(&workspace_id)
                .is_some_and(|saved_path| saved_path == &path)
            {
                continue;
            }
            self.persisted_remote_paths
                .insert(workspace_id, path.clone());
            let session = cx.global::<Storage>().session.clone();
            let task_profile_id = profile_id.clone();
            log::debug!(
                "SFTP 远程目录路径变化，准备保存: session={}, path={}",
                profile_id,
                path
            );
            cx.spawn(async move |_this, _cx| {
                let result = tokio::task::spawn_blocking(move || {
                    session.update_sftp_remote_path(&task_profile_id, &path)
                })
                .await;
                match result {
                    Ok(Ok(())) => {
                        log::debug!("SFTP 远程目录保存完成: session={profile_id}");
                    }
                    Ok(Err(error)) => {
                        log::warn!("保存 SFTP 远程目录失败，会话 {profile_id}: {error:#}");
                    }
                    Err(error) => {
                        log::warn!("保存 SFTP 远程目录任务失败，会话 {profile_id}: {error}");
                    }
                }
            })
            .detach();
        }
    }

    pub(super) fn connect(
        &mut self,
        workspace_id: String,
        profile: SessionProfile,
        initial_remote_path: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.close(&workspace_id);
        self.remote_selection.clear();
        self.remote_list_state.reset_with_uniform_height(0, px(38.));

        let model = Arc::new(SftpModel {
            snapshot: RwLock::new(SftpSnapshot::default()),
            transfers: self.transfers.clone(),
            cancelled_transfers: RwLock::new(Default::default()),
            updates: self.updates.clone(),
            directory_updates: self.directory_updates.clone(),
            status_updates: self.status_updates.clone(),
            transfer_ui_throttle: self.transfer_ui_throttle.clone(),
        });
        let (commands, command_receiver) = mpsc::unbounded_channel();
        let task_model = model.clone();
        let profile_id = profile.id.clone();
        let profile_ip = profile.host.clone();
        let profile_title = profile.name.clone();
        let task_initial_remote_path = initial_remote_path.clone();
        let task = tokio::spawn(async move {
            if let Err(error) = remote::run_sftp(
                profile,
                task_initial_remote_path,
                command_receiver,
                task_model.clone(),
            )
            .await
            {
                task_model.set_failed(format!("{error:#}"));
            }
        });
        if let Some(path) = initial_remote_path {
            self.persisted_remote_paths
                .insert(workspace_id.clone(), path);
        }
        let should_restore_local_directory =
            self.selected_workspace_id.as_deref() == Some(workspace_id.as_str());
        self.runtimes.insert(
            workspace_id.clone(),
            SftpRuntime {
                profile_id,
                profile_ip,
                profile_title,
                model,
                commands,
                task,
            },
        );
        self.updates.notify_one();
        if should_restore_local_directory {
            self.restore_local_directory(&workspace_id, cx);
        }
    }

    pub(super) fn close(&mut self, workspace_id: &str) {
        self.stop_local_watchers_for_workspace(workspace_id);
        self.persisted_remote_paths.remove(workspace_id);
        if let Some(runtime) = self.runtimes.remove(workspace_id) {
            let _ = runtime.commands.send(SftpCommand::Disconnect);
            runtime.task.abort();
        }
    }

    pub(super) fn load_directory(&mut self, path: String) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        let _ = self.load_directory_for_workspace(&workspace_id, path);
    }

    pub(super) fn load_directory_for_workspace(
        &mut self,
        workspace_id: &str,
        path: String,
    ) -> Result<(), String> {
        let is_selected = self.selected_workspace_id.as_deref() == Some(workspace_id);
        if is_selected {
            self.remote_selection.clear();
            self.remote_list_state.reset_with_uniform_height(0, px(38.));
        }
        let runtime = self
            .runtimes
            .get(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        runtime.model.set_loading();
        runtime
            .commands
            .send(SftpCommand::LoadDirectory(path))
            .map_err(|_| {
                runtime.model.set_error("SFTP 连接已关闭".to_owned());
                "SFTP 连接已关闭".to_owned()
            })
    }

    pub(super) fn upload_file(&mut self, local_path: PathBuf, cx: &mut Context<Self>) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        self.upload_file_for_workspace(&workspace_id, local_path, cx);
    }

    pub(super) fn upload_file_for_workspace(
        &mut self,
        workspace_id: &str,
        local_path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        let Some(runtime) = self.runtimes.get(workspace_id) else {
            return None;
        };
        let snapshot = runtime.model.snapshot();
        let Some(file_name) = local_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            return None;
        };
        let remote_path = remote::join_remote_path(&snapshot.path, &file_name);
        self.upload_file_to_remote_path(workspace_id, local_path, remote_path, cx)
    }

    pub(super) fn upload_file_to_remote_path(
        &mut self,
        workspace_id: &str,
        local_path: PathBuf,
        remote_path: String,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        let Some(runtime) = self.runtimes.get(workspace_id) else {
            return None;
        };
        let snapshot = runtime.model.snapshot();
        let commands = runtime.commands.clone();
        let Some(file_name) = local_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            return None;
        };
        let request = TransferRequest::Upload {
            workspace_id: workspace_id.to_owned(),
            local_path: local_path.clone(),
            is_directory: false,
        };
        let transfer_id = self.push_transfer(file_name, "上传", remote_path.clone(), request, cx);
        if commands
            .send(SftpCommand::Upload {
                transfer_id,
                local_path,
                remote_path,
                refresh_path: snapshot.path,
            })
            .is_err()
        {
            self.fail_queued_transfer(transfer_id, cx);
            return None;
        }
        Some(transfer_id)
    }

    pub(super) fn download_file(
        &mut self,
        remote_path: String,
        file_name: String,
        total_size: u64,
        is_directory: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        self.download_file_for_workspace(
            &workspace_id,
            remote_path,
            file_name,
            total_size,
            is_directory,
            cx,
        );
    }

    pub(super) fn download_file_for_workspace(
        &mut self,
        workspace_id: &str,
        remote_path: String,
        file_name: String,
        total_size: u64,
        is_directory: bool,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        let Some(runtime) = self.runtimes.get(workspace_id) else {
            return None;
        };
        let commands = runtime.commands.clone();
        let local_directory = self.local.path.clone();
        let local_path = local_directory.join(&file_name);
        let request = TransferRequest::Download {
            workspace_id: workspace_id.to_owned(),
            remote_path: remote_path.clone(),
            file_name: file_name.clone(),
            total_size,
            is_directory,
        };
        let transfer_id = self.push_transfer(
            file_name,
            "下载",
            local_path.display().to_string(),
            request,
            cx,
        );
        let (complete, completion) = oneshot::channel();
        if commands
            .send(SftpCommand::Download {
                transfer_id,
                remote_path,
                local_path,
                total_size,
                is_directory,
                complete,
            })
            .is_err()
        {
            self.fail_queued_transfer(transfer_id, cx);
            return None;
        }
        cx.spawn(async move |this, cx| {
            if completion.await.unwrap_or(false) {
                let _ = this.update(cx, |this, cx| {
                    if this.local.path == local_directory {
                        this.load_local_directory(local_directory, cx);
                    }
                });
            }
        })
        .detach();
        Some(transfer_id)
    }

    fn push_transfer(
        &mut self,
        name: String,
        direction: &str,
        target: String,
        request: TransferRequest,
        cx: &mut Context<Self>,
    ) -> u64 {
        let transfer_id = self.next_transfer_id;
        self.next_transfer_id = self.next_transfer_id.wrapping_add(1).max(1);
        self.transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(TransferRecord {
                id: transfer_id,
                name,
                direction: direction.to_owned(),
                target,
                request,
                progress: 0.,
                transferred_bytes: 0,
                total_bytes: 0,
                speed: 0,
                started_at: None,
                speed_updated_at: None,
                status: "等待中".to_owned(),
                error: None,
            });
        cx.notify();
        transfer_id
    }

    fn fail_queued_transfer(&self, transfer_id: u64, cx: &mut Context<Self>) {
        if let Some(transfer) = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            transfer.status = "失败".to_owned();
            transfer.error = Some("传输任务未能加入 SFTP 队列".to_owned());
        }
        cx.notify();
    }

    pub(super) fn cancel_transfer(
        &mut self,
        action: &CancelTransfer,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find(|transfer| transfer.id == action.0)
            .cloned()
        else {
            return;
        };
        if let Some(runtime) = self.runtimes.get(record.request.workspace_id()) {
            runtime.model.request_cancel(record.id);
        }
        cx.notify();
    }

    pub(super) fn retry_transfer(
        &mut self,
        action: &RetryTransfer,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find(|transfer| transfer.id == action.0)
            .cloned()
        else {
            return;
        };
        if record.status != "失败" && record.status != "已取消" {
            return;
        }
        let request = record.request;
        if let Some(runtime) = self.runtimes.get(request.workspace_id()) {
            runtime.model.clear_cancel(record.id);
        }
        match request {
            TransferRequest::Upload {
                workspace_id,
                local_path,
                ..
            } => {
                self.upload_file_for_workspace(&workspace_id, local_path, cx);
            }
            TransferRequest::Download {
                workspace_id,
                remote_path,
                file_name,
                total_size,
                is_directory,
            } => {
                self.download_file_for_workspace(
                    &workspace_id,
                    remote_path,
                    file_name,
                    total_size,
                    is_directory,
                    cx,
                );
            }
        }
    }

    pub(super) fn upload_local_entry(
        &mut self,
        action: &UploadLocalEntry,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for path in &action.0 {
            self.upload_file(path.clone(), cx);
        }
    }

    pub(super) fn download_remote_entry(
        &mut self,
        action: &DownloadRemoteEntry,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for item in &action.items {
            self.download_file(
                item.path.clone(),
                item.name.clone(),
                item.size,
                item.is_directory,
                cx,
            );
        }
    }
}
