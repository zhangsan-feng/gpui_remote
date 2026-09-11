mod delete;
mod local_view;
mod watcher;

use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::domain::session::SessionProfile;
use gpui_kit::*;

use super::{
    CancelTransfer, DownloadRemoteEntry, RetryTransfer, SftpModel, SftpProjection, SftpSnapshot,
    SftpView, UploadLocalEntry,
};

impl SftpModel {
    pub(super) fn snapshot(&self) -> SftpSnapshot {
        self.snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub(in crate::gui::workspace::sftp) fn replace_snapshot(&self, next_snapshot: SftpSnapshot) {
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *snapshot = next_snapshot;
        }
    }
}

impl SftpView {
    pub(super) fn persist_remote_directory(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        let Some(profile_id) = self
            .projections
            .get(&workspace_id)
            .map(|projection| projection.profile_id.clone())
        else {
            return;
        };
        if path.is_empty()
            || self
                .persisted_remote_paths
                .get(&workspace_id)
                .is_some_and(|saved_path| saved_path == path)
        {
            return;
        }
        self.persisted_remote_paths
            .insert(workspace_id.clone(), path.to_owned());
        let path = path.to_owned();
        let application = self.application.clone();
        log::debug!(
            "SFTP 远程目录路径变化，准备保存: session={}, path={}",
            profile_id,
            path
        );
        cx.spawn(async move |_this, _cx| {
            match application
                .persist_sftp_remote_path(workspace_id, path)
                .await
            {
                Ok(()) => log::debug!("SFTP 远程目录保存完成: 会话 {profile_id}"),
                Err(error) => log::warn!("保存 SFTP 远程目录失败，会话 {profile_id}: {error}"),
            }
        })
        .detach();
    }

    pub(super) fn connect_projection(&mut self, workspace_id: String, profile: SessionProfile) {
        self.close(&workspace_id);
        self.remote_selection.clear();
        self.remote_list_state.reset_with_uniform_height(0, px(38.));

        let model = Arc::new(SftpModel {
            snapshot: RwLock::new(SftpSnapshot::default()),
        });
        let profile_id = profile.id.clone();
        let profile_ip = profile.host.clone();
        let profile_title = profile.name.clone();
        self.projections.insert(
            workspace_id.clone(),
            SftpProjection {
                profile_id,
                profile_ip,
                profile_title,
                model,
            },
        );
        self.updates.notify_one();
    }

    pub(super) fn close(&mut self, workspace_id: &str) {
        self.stop_local_watchers_for_workspace(workspace_id);
        self.remote_revisions.remove(workspace_id);
        self.local_restore_requests.remove(workspace_id);
        self.persisted_remote_paths.remove(workspace_id);
        self.projections.remove(workspace_id);
    }

    pub(super) fn load_directory(&mut self, path: String, cx: &mut Context<Self>) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        let _ = self.load_directory_for_workspace(&workspace_id, path, cx);
    }

    pub(super) fn load_directory_for_workspace(
        &mut self,
        workspace_id: &str,
        path: String,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let is_selected = self.selected_workspace_id.as_deref() == Some(workspace_id);
        if is_selected {
            self.remote_selection.clear();
            self.remote_list_state.reset_with_uniform_height(0, px(38.));
        }
        let projection = self
            .projections
            .get(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        let profile_ip = projection.profile_ip.clone();
        let profile_title = projection.profile_title.clone();
        let application = self.application.clone();
        let workspace_id = workspace_id.to_owned();
        cx.spawn(async move |_this, _cx| {
            if let Err(error) = application
                .change_sftp_remote_directory(workspace_id, profile_ip, profile_title, path)
                .await
            {
                log::warn!("SFTP 远程目录请求失败: {error}");
            }
        })
        .detach();
        Ok(())
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
    ) {
        if !self.projections.contains_key(workspace_id) {
            return;
        }
        let application = self.application.clone();
        let task_workspace_id = workspace_id.to_owned();
        let path_text = local_path.display().to_string();
        cx.spawn(async move |this, cx| {
            match application
                .upload_sftp(task_workspace_id, vec![path_text])
                .await
            {
                Ok(_) => {
                    let _ = this.update(cx, |this, cx| {
                        this.sync_application_state();
                        cx.notify();
                    });
                }
                Err(error) => {
                    log::warn!("SFTP 上传请求失败: {error}");
                }
            }
        })
        .detach();
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
    ) {
        if !self.projections.contains_key(workspace_id) {
            return;
        }
        let application = self.application.clone();
        let task_workspace_id = workspace_id.to_owned();
        cx.spawn(async move |this, cx| {
            match application
                .download_sftp(task_workspace_id, vec![remote_path])
                .await
            {
                Ok(_) => {
                    let _ = this.update(cx, |this, cx| {
                        this.sync_application_state();
                        cx.notify();
                    });
                }
                Err(error) => {
                    log::warn!(
                        "SFTP 下载请求失败: file={file_name}, size={total_size}, directory={is_directory}, error={error}"
                    );
                }
            }
        })
        .detach();
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
        let application = self.application.clone();
        let workspace_id = record.workspace_id.clone();
        cx.spawn(async move |this, cx| {
            if let Err(error) = application
                .cancel_sftp_transfer(workspace_id, record.id)
                .await
            {
                log::warn!("取消 SFTP 传输失败: {error}");
            }
            let _ = this.update(cx, |this, cx| {
                this.sync_application_state();
                cx.notify();
            });
        })
        .detach();
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
        let workspace_id = record.workspace_id.clone();
        let application = self.application.clone();
        cx.spawn(async move |this, cx| {
            if let Err(error) = application
                .retry_sftp_transfer(workspace_id, record.id)
                .await
            {
                log::warn!("重试 SFTP 传输失败: {error}");
            }
            let _ = this.update(cx, |this, cx| {
                this.sync_application_state();
                cx.notify();
            });
        })
        .detach();
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
