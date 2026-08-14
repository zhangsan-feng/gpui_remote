use std::{path::PathBuf, sync::Arc};

use gpui::*;
use tokio::sync::Notify;

use crate::{
    application::agent_mcp::{
        SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo, SftpTransferSummary,
    },
    domain::{session::Protocol, terminal::TerminalStatus},
    global_state::{GlobalEvent, read_global_state},
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
                    this.connect(workspace_id.clone(), profile.clone());
                }
                GlobalEvent::SelectWorkspaceSession(workspace_id) => {
                    if this.selected_workspace_id == *workspace_id {
                        return;
                    }
                    this.selected_workspace_id = workspace_id.clone();
                    this.remote_list_state.reset_with_uniform_height(0, px(38.));
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
