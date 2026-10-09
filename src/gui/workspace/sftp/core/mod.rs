mod delete;
mod local_view;
mod watcher;

use std::path::PathBuf;

use crate::data_context::{SftpWorkspaceSnapshot, WorkspaceSummary};
use gpui_kit::*;

use super::{
    CancelTransfer, DownloadRemoteEntry, RetryTransfer, SftpProjection, SftpView, UploadLocalEntry,
};

impl SftpView {
    pub(super) fn initialize_projection(
        &mut self,
        workspace_id: String,
        profile: WorkspaceSummary,
    ) {
        let existing = self.projections.contains_key(&workspace_id);
        log::debug!(
            "SFTP GUI projection connecting: workspace_id={workspace_id}, profile_id={}, existing={}",
            profile.profile_id,
            existing
        );
        if existing {
            log::debug!(
                "SFTP GUI projection event already handled, skipping: workspace_id={workspace_id}"
            );
            return;
        }
        self.close(&workspace_id);
        self.remote_selection.clear();
        self.remote_list_state.reset_with_uniform_height(0, px(38.));

        let profile_ip = profile.host.clone();
        let profile_title = profile.title.clone();
        self.projections.insert(
            workspace_id.clone(),
            SftpProjection {
                profile_ip,
                profile_title,
                snapshot: SftpWorkspaceSnapshot::default(),
                local_navigation_tail: None,
                remote_navigation_tail: None,
            },
        );
    }

    pub(super) fn close(&mut self, workspace_id: &str) {
        self.clear_local_watch_projection_for_workspace(workspace_id);
        self.local_restore_requests.remove(workspace_id);
        self.observed_sftp_revisions.remove(workspace_id);
        self.local_back_history.remove(workspace_id);
        self.remote_back_history.remove(workspace_id);
        self.projections.remove(workspace_id);
    }

    pub(super) fn change_remote_directory(&mut self, path: String, cx: &mut Context<Self>) {
        self.navigate_remote_directory(path, true, cx);
    }

    pub(super) fn change_remote_directory_without_history(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        self.navigate_remote_directory(path, false, cx);
    }

    // Parent, back, double-click and dialog navigation all use this entry point.
    fn navigate_remote_directory(
        &mut self,
        path: String,
        record_history: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        if record_history {
            let current_path = self
                .projections
                .get(&workspace_id)
                .map(|projection| projection.snapshot.remote.path.clone())
                .unwrap_or_default();
            if !current_path.is_empty() && current_path != path {
                self.push_remote_back_path(&workspace_id, current_path);
            }
        }
        let _ = self.change_remote_directory_for_workspace(&workspace_id, path, cx);
    }

    pub(super) fn can_go_remote_back(&self) -> bool {
        self.selected_workspace_id
            .as_deref()
            .and_then(|workspace_id| self.remote_back_history.get(workspace_id))
            .is_some_and(|history| !history.is_empty())
    }

    pub(super) fn pop_remote_back_path(&mut self) -> Option<String> {
        let workspace_id = self.selected_workspace_id.as_deref()?;
        self.remote_back_history
            .get_mut(workspace_id)
            .and_then(Vec::pop)
    }

    fn push_remote_back_path(&mut self, workspace_id: &str, path: String) {
        let history = self
            .remote_back_history
            .entry(workspace_id.to_owned())
            .or_default();
        if history.last() != Some(&path) {
            history.push(path);
        }
    }

    pub(super) fn change_remote_directory_for_workspace(
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
            .get_mut(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        let profile_ip = projection.profile_ip.clone();
        let profile_title = projection.profile_title.clone();
        // Reserve order on the GUI thread before work enters Tokio's scheduler.
        let previous = projection.remote_navigation_tail.take();
        let (complete, next) = tokio::sync::oneshot::channel();
        projection.remote_navigation_tail = Some(next);
        let application = crate::application::APPLICATION.clone();
        let workspace_id = workspace_id.to_owned();
        cx.spawn(async move |_this, _cx| {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let result = crate::global_state::run_application(async move {
                application
                    .sftp
                    .change_remote_directory_checked(workspace_id, profile_ip, profile_title, path)
                    .await
            })
            .await;
            let _ = complete.send(());
            if let Err(error) = result {
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
        let application = crate::application::APPLICATION.clone();
        let task_workspace_id = workspace_id.to_owned();
        let path_text = local_path.display().to_string();
        cx.spawn(async move |this, cx| {
            match crate::global_state::run_application(async move {
                application
                    .sftp
                    .upload_checked(task_workspace_id, vec![path_text])
                    .await
            })
            .await
            {
                Ok(_) => {
                    let _ = this.update(cx, |this, cx| {
                        this.refresh_from_context(cx);
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
        let application = crate::application::APPLICATION.clone();
        let task_workspace_id = workspace_id.to_owned();
        cx.spawn(async move |this, cx| {
            match crate::global_state::run_application(async move { application
                .sftp.download_checked(task_workspace_id, vec![remote_path])
                .await }).await
            {
                Ok(_) => {
                    let _ = this.update(cx, |this, cx| {
                        this.refresh_from_context(cx);
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
            .iter()
            .find(|transfer| transfer.id == action.0)
            .cloned()
        else {
            return;
        };
        let application = crate::application::APPLICATION.clone();
        let workspace_id = record.workspace_id.clone();
        cx.spawn(async move |this, cx| {
            if let Err(error) = crate::global_state::run_application(async move {
                application
                    .sftp
                    .cancel_transfer_checked(workspace_id, record.id)
                    .await
            })
            .await
            {
                log::warn!("取消 SFTP 传输失败: {error}");
            }
            let _ = this.update(cx, |this, cx| {
                this.refresh_from_context(cx);
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
        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            if let Err(error) = crate::global_state::run_application(async move {
                application
                    .sftp
                    .retry_transfer_checked(workspace_id, record.id)
                    .await
            })
            .await
            {
                log::warn!("重试 SFTP 传输失败: {error}");
            }
            let _ = this.update(cx, |this, cx| {
                this.refresh_from_context(cx);
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
