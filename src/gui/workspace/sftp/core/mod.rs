mod conn;
mod local;
mod remote;

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};

use gpui::*;
use tokio::sync::{mpsc, oneshot};

use crate::domain::session::SessionProfile;

use super::{
    CancelTransfer, DeleteLocalEntry, DeleteRemoteEntry, DownloadRemoteEntry, RetryTransfer,
    SftpCommand, SftpModel, SftpRuntime, SftpSnapshot, SftpStatus, SftpView, TransferRecord,
    TransferRequest, UploadLocalEntry,
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
        self.updates.notify_waiters();
        if status_changed {
            self.status_updates.notify_waiters();
        }
    }

    fn set_connected(&self, path: String, entries: Vec<super::SftpEntry>) {
        self.update(
            |snapshot| {
                snapshot.status = SftpStatus::Connected;
                snapshot.path = path;
                snapshot.entries = Arc::new(entries);
                snapshot.loading = false;
                snapshot.error = None;
            },
            true,
        );
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
        self.update(
            |snapshot| {
                snapshot.path = path;
                snapshot.entries = Arc::new(entries);
                snapshot.loading = false;
                snapshot.error = None;
            },
            false,
        );
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
        status: impl Into<String>,
    ) {
        let status = status.into();
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
                transfer.started_at = None;
                transfer.speed_updated_at = None;
            } else if transferred > 0 {
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
        self.updates.notify_waiters();
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
        self.updates.notify_waiters();
    }
}

impl SftpView {
    pub(super) fn load_local_directory(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.local_selection.clear();
        self.local.path = path.clone();
        self.local.loading = true;
        self.local.error = None;
        self.local_list_state.reset_with_uniform_height(0, px(38.));
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = tokio::task::spawn_blocking(move || local::read_local_directory(&path))
                .await
                .map_err(|error| anyhow::anyhow!("读取本地目录任务失败: {error}"))
                .and_then(|result| result);
            let _ = this.update(cx, |this, cx| {
                this.local.loading = false;
                match result {
                    Ok((path, entries)) => {
                        this.local.path = path;
                        this.local.entries = Arc::new(entries);
                        this.local.error = None;
                    }
                    Err(error) => {
                        this.local.error = Some(format!("{error:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn connect(&mut self, workspace_id: String, profile: SessionProfile) {
        self.close(&workspace_id);
        self.remote_selection.clear();
        self.remote_list_state.reset_with_uniform_height(0, px(38.));

        let model = Arc::new(SftpModel {
            snapshot: RwLock::new(SftpSnapshot::default()),
            transfers: self.transfers.clone(),
            cancelled_transfers: RwLock::new(Default::default()),
            updates: self.updates.clone(),
            status_updates: self.status_updates.clone(),
        });
        let (commands, command_receiver) = mpsc::unbounded_channel();
        let task_model = model.clone();
        let task = tokio::spawn(async move {
            if let Err(error) =
                remote::run_sftp(profile, command_receiver, task_model.clone()).await
            {
                task_model.set_failed(format!("{error:#}"));
            }
        });
        self.runtimes.insert(
            workspace_id,
            SftpRuntime {
                model,
                commands,
                task,
            },
        );
    }

    pub(super) fn close(&mut self, workspace_id: &str) {
        if let Some(runtime) = self.runtimes.remove(workspace_id) {
            let _ = runtime.commands.send(SftpCommand::Disconnect);
            runtime.task.abort();
        }
    }

    pub(super) fn load_directory(&mut self, path: String) {
        self.remote_selection.clear();
        let Some(runtime) = self
            .selected_workspace_id
            .as_deref()
            .and_then(|workspace_id| self.runtimes.get(workspace_id))
        else {
            return;
        };
        self.remote_list_state.reset_with_uniform_height(0, px(38.));
        runtime.model.set_loading();
        if runtime
            .commands
            .send(SftpCommand::LoadDirectory(path))
            .is_err()
        {
            runtime.model.set_error("SFTP 连接已关闭".to_owned());
        }
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
    ) -> bool {
        let Some(runtime) = self.runtimes.get(workspace_id) else {
            return false;
        };
        let snapshot = runtime.model.snapshot();
        let commands = runtime.commands.clone();
        let Some(file_name) = local_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            return false;
        };
        let Ok(metadata) = fs::metadata(&local_path) else {
            return false;
        };
        if !metadata.is_file() && !metadata.is_dir() {
            return false;
        }
        let remote_path = remote::join_remote_path(&snapshot.path, &file_name);
        let request = TransferRequest::Upload {
            workspace_id: workspace_id.to_owned(),
            local_path: local_path.clone(),
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
            return false;
        }
        true
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
    ) -> bool {
        let Some(runtime) = self.runtimes.get(workspace_id) else {
            return false;
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
            return false;
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
        true
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
                speed: 0,
                started_at: None,
                speed_updated_at: None,
                status: "等待中".to_owned(),
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

    pub(super) fn delete_local_entry(
        &mut self,
        action: &DeleteLocalEntry,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = action.0.clone();
        let current_directory = self.local.path.clone();
        self.local.error = None;
        cx.spawn(async move |this, cx| {
            let result = tokio::task::spawn_blocking(move || local::delete_local_path(&path))
                .await
                .map_err(|error| anyhow::anyhow!("删除本地路径任务失败: {error}"))
                .and_then(|result| result);
            let _ = this.update(cx, |this, cx| {
                if this.local.path != current_directory {
                    return;
                }
                match result {
                    Ok(()) => this.load_local_directory(current_directory, cx),
                    Err(error) => {
                        this.local.error = Some(format!("{error:#}"));
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    pub(super) fn delete_remote_entry(
        &mut self,
        action: &DeleteRemoteEntry,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.selected_snapshot() else {
            return;
        };
        let Some(workspace_id) = self.selected_workspace_id.as_deref() else {
            return;
        };
        let Some(runtime) = self.runtimes.get(workspace_id) else {
            return;
        };
        runtime.model.set_loading();
        if runtime
            .commands
            .send(SftpCommand::Delete {
                path: action.path.clone(),
                is_directory: action.is_directory,
                refresh_path: snapshot.path,
            })
            .is_err()
        {
            runtime.model.set_error("SFTP 连接已关闭".to_owned());
        }
        cx.notify();
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

pub(super) fn default_desktop_path() -> PathBuf {
    let profile = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let desktop = profile.join("Desktop");
    if desktop.is_dir() {
        desktop
    } else {
        profile
    }
}
