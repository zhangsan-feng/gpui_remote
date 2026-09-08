use std::{path::PathBuf, sync::Arc};

use gpui_kit::*;
use tokio::sync::Notify;

use crate::{
    application::agent_mcp::{
        SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo, SftpTransferSummary,
        SftpWatchSummary,
    },
    domain::{session::Protocol, terminal::TerminalStatus},
    global_state::{GlobalEvent, read_global_state},
    infrastructure::storage::Storage,
};

use super::{SftpStatus, SftpView};

impl SftpView {
    pub(in crate::gui::workspace) fn mcp_local_directory(&self) -> SftpDirectorySummary {
        SftpDirectorySummary {
            path: self.local.path.display().to_string(),
            entries: self
                .local
                .entries
                .iter()
                .map(|entry| SftpEntrySummary {
                    name: entry.name.clone(),
                    path: entry.path.display().to_string(),
                    is_directory: entry.is_directory,
                    size: entry.size,
                })
                .collect(),
            loading: self.local.loading,
            error: self.local.error.clone(),
        }
    }

    pub(in crate::gui::workspace) fn mcp_change_local_directory(
        &mut self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        self.validate_sftp_session(&workspace_id, &ip, &title)?;
        if self.selected_workspace_id.as_deref() != Some(workspace_id.as_str()) {
            return Err(format!("SFTP 会话未选中: {workspace_id}，请先切换到该会话"));
        }
        let path = PathBuf::from(path);
        if !path.is_dir() {
            return Err(format!("本地目录不存在: {}", path.display()));
        }
        self.load_local_directory(path, cx);
        Ok(())
    }

    pub(in crate::gui::workspace) fn mcp_remote_directory(
        &self,
        workspace_id: &str,
    ) -> Result<SftpDirectorySummary, String> {
        let runtime = self
            .runtimes
            .get(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        let snapshot = runtime.model.snapshot();
        Ok(SftpDirectorySummary {
            path: snapshot.path,
            entries: snapshot
                .entries
                .iter()
                .map(|entry| SftpEntrySummary {
                    name: entry.name.clone(),
                    path: entry.path.clone(),
                    is_directory: entry.is_directory,
                    size: entry.size,
                })
                .collect(),
            loading: snapshot.loading,
            error: snapshot.error,
        })
    }

    pub(in crate::gui::workspace) fn mcp_change_remote_directory(
        &mut self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        self.validate_sftp_session(&workspace_id, &ip, &title)?;
        self.load_directory_for_workspace(&workspace_id, path)?;
        if self.selected_workspace_id.as_deref() == Some(workspace_id.as_str()) {
            cx.notify();
        }
        Ok(())
    }

    pub(in crate::gui::workspace) fn mcp_upload(
        &mut self,
        workspace_id: &str,
        local_paths: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Result<SftpTransferSummary, String> {
        if local_paths.is_empty() {
            return Err("至少需要一个本地路径".to_owned());
        }
        if !self.runtimes.contains_key(workspace_id) {
            return Err(format!("SFTP 会话不存在: {workspace_id}"));
        }

        let transfer_ids = local_paths
            .into_iter()
            .filter_map(|path| {
                self.upload_file_for_workspace(workspace_id, PathBuf::from(path), cx)
            })
            .collect::<Vec<_>>();
        if transfer_ids.is_empty() {
            return Err("没有可加入队列的本地文件或目录".to_owned());
        }
        Ok(SftpTransferSummary {
            queued: transfer_ids.len(),
            transfers: self.mcp_transfer_infos(&transfer_ids),
        })
    }

    pub(in crate::gui::workspace) fn mcp_download(
        &mut self,
        workspace_id: &str,
        remote_paths: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Result<SftpTransferSummary, String> {
        if remote_paths.is_empty() {
            return Err("至少需要一个远程路径".to_owned());
        }
        let runtime = self
            .runtimes
            .get(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        let snapshot = runtime.model.snapshot();
        let entries = remote_paths
            .iter()
            .map(|path| {
                snapshot
                    .entries
                    .iter()
                    .find(|entry| entry.path == *path)
                    .cloned()
                    .ok_or_else(|| format!("当前远程目录不存在路径: {path}"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let transfer_ids = entries
            .into_iter()
            .filter_map(|entry| {
                self.download_file_for_workspace(
                    workspace_id,
                    entry.path.clone(),
                    entry.name.clone(),
                    entry.size,
                    entry.is_directory,
                    cx,
                )
            })
            .collect::<Vec<_>>();
        if transfer_ids.is_empty() {
            return Err("没有可加入队列的远程文件或目录".to_owned());
        }
        Ok(SftpTransferSummary {
            queued: transfer_ids.len(),
            transfers: self.mcp_transfer_infos(&transfer_ids),
        })
    }

    pub(in crate::gui::workspace) fn mcp_transfers(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SftpTransferInfo>, String> {
        if !self.runtimes.contains_key(workspace_id) {
            return Err(format!("SFTP 会话不存在: {workspace_id}"));
        }
        let transfers = self
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(transfers
            .iter()
            .filter(|transfer| transfer.request.workspace_id() == workspace_id)
            .map(transfer_info)
            .collect())
    }

    pub(in crate::gui::workspace) fn mcp_watch_local(
        &mut self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
        cx: &mut Context<Self>,
    ) -> Result<SftpWatchSummary, String> {
        self.validate_sftp_session(&workspace_id, &ip, &title)?;
        self.watch_local_path_for_workspace(&workspace_id, PathBuf::from(local_path), cx)
    }

    pub(in crate::gui::workspace) fn mcp_stop_watching_local(
        &mut self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        self.validate_sftp_session(&workspace_id, &ip, &title)?;
        let result =
            self.stop_watching_local_path_for_workspace(&workspace_id, &PathBuf::from(local_path));
        if result.is_ok() {
            cx.notify();
        }
        result
    }

    pub(in crate::gui::workspace) fn mcp_list_local_watches(
        &self,
        workspace_id: &str,
        ip: &str,
        title: &str,
    ) -> Result<Vec<SftpWatchSummary>, String> {
        self.validate_sftp_session(workspace_id, ip, title)?;
        self.local_watch_summaries(workspace_id)
    }

    fn validate_sftp_session(
        &self,
        workspace_id: &str,
        ip: &str,
        title: &str,
    ) -> Result<(), String> {
        let runtime = self
            .runtimes
            .get(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        if runtime.profile_ip != ip || runtime.profile_title != title {
            return Err(format!(
                "SFTP 会话信息不匹配: {workspace_id}，请确认 ip 和 title"
            ));
        }
        Ok(())
    }

    fn mcp_transfer_infos(&self, transfer_ids: &[u64]) -> Vec<SftpTransferInfo> {
        let transfers = self
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        transfers
            .iter()
            .filter(|transfer| transfer_ids.contains(&transfer.id))
            .map(transfer_info)
            .collect()
    }

    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        cx.subscribe(&global_state, |this, _, event, cx| {
            match event {
                GlobalEvent::OpenWorkspaceSession(workspace_id, profile)
                    if profile.protocol == Protocol::Sftp =>
                {
                    let initial_remote_path = match cx
                        .global::<Storage>()
                        .session
                        .sftp_state(&profile.id)
                    {
                        Ok(state) => state.and_then(|state| state.remote_path),
                        Err(error) => {
                            log::warn!("读取 SFTP 远程目录失败，会话 {}: {error:#}", profile.id);
                            None
                        }
                    };
                    this.connect(
                        workspace_id.clone(),
                        profile.clone(),
                        initial_remote_path,
                        cx,
                    );
                }
                GlobalEvent::SelectWorkspaceSession(workspace_id) => {
                    if this.selected_workspace_id == *workspace_id {
                        return;
                    }
                    this.selected_workspace_id = workspace_id.clone();
                    this.remote_list_state.reset_with_uniform_height(0, px(38.));
                    let sftp_workspace_id = workspace_id
                        .as_deref()
                        .map(str::to_owned)
                        .filter(|workspace_id| this.runtimes.contains_key(workspace_id));
                    if let Some(workspace_id) = sftp_workspace_id.as_deref() {
                        this.restore_local_directory(workspace_id, cx);
                    }
                }
                GlobalEvent::CloseWorkspaceSession { workspace_id } => {
                    this.close(workspace_id);
                }
                _ => return,
            }
            cx.notify();
        })
        .detach();
    }

    pub(in crate::gui::workspace) fn status_updates(&self) -> Arc<Notify> {
        self.status_updates.clone()
    }

    pub(in crate::gui::workspace) fn connection_status(
        &self,
        workspace_id: &str,
    ) -> Option<TerminalStatus> {
        let status = self.runtimes.get(workspace_id)?.model.snapshot().status;
        Some(match status {
            SftpStatus::Connecting => TerminalStatus::Connecting,
            SftpStatus::Connected => TerminalStatus::Connected,
            SftpStatus::Disconnected => TerminalStatus::Disconnected,
            SftpStatus::Failed => TerminalStatus::Failed,
        })
    }
}

fn transfer_info(transfer: &super::TransferRecord) -> SftpTransferInfo {
    let (source, is_directory) = match &transfer.request {
        super::TransferRequest::Upload {
            local_path,
            is_directory,
            ..
        } => (local_path.display().to_string(), *is_directory),
        super::TransferRequest::Download {
            remote_path,
            is_directory,
            ..
        } => (remote_path.clone(), *is_directory),
    };
    SftpTransferInfo {
        id: transfer.id,
        workspace_id: transfer.request.workspace_id().to_owned(),
        name: transfer.name.clone(),
        direction: transfer.direction.clone(),
        source,
        target: transfer.target.clone(),
        is_directory,
        progress: transfer.progress,
        transferred_bytes: transfer.transferred_bytes,
        total_bytes: transfer.total_bytes,
        speed_bytes_per_second: transfer.speed,
        status: transfer.status.clone(),
        error: transfer.error.clone(),
    }
}
