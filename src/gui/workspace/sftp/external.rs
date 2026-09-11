use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

use gpui_kit::*;

use crate::{
    application::model::{SftpDirectorySummary, SftpTransferInfo},
    domain::{session::Protocol, terminal::TerminalStatus},
    global_state::{GlobalEvent, read_global_state},
};

use super::{SftpStatus, SftpView};

impl SftpView {
    pub(super) fn sync_application_state(&mut self) {
        let workspace_ids = self.projections.keys().cloned().collect::<Vec<_>>();
        for workspace_id in workspace_ids {
            let Some(projection) = self.projections.get(&workspace_id) else {
                continue;
            };
            if let Ok(watches) = self.application.list_sftp_local_watches(
                workspace_id.clone(),
                projection.profile_ip.clone(),
                projection.profile_title.clone(),
            ) {
                self.local_watchers.insert(
                    workspace_id.clone(),
                    watches
                        .into_iter()
                        .map(|watch| (PathBuf::from(&watch.local_path), watch))
                        .collect::<HashMap<_, _>>(),
                );
            }
            let Ok(revision) = self.application.sftp_revision(&workspace_id) else {
                continue;
            };
            if self.remote_revisions.get(&workspace_id) == Some(&revision) {
                continue;
            }
            if let Ok(summary) = self.application.sftp_snapshot(&workspace_id) {
                let status = self
                    .application
                    .sftp_connection_status(&workspace_id)
                    .ok()
                    .map(to_sftp_status)
                    .unwrap_or(SftpStatus::Connecting);
                projection
                    .model
                    .replace_snapshot(to_sftp_snapshot(summary, status));
                self.remote_revisions.insert(workspace_id, revision);
            }
        }

        if let Some(workspace_id) = self.selected_workspace_id.as_deref() {
            if let Ok(summary) = self.application.sftp_local_snapshot(workspace_id) {
                self.local = to_local_snapshot(summary);
            }
            if let Ok(transfers) = self.application.sftp_transfers_snapshot(workspace_id) {
                *self
                    .transfers
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) =
                    transfers.into_iter().map(to_transfer_record).collect();
            }
        }
    }

    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        cx.subscribe(&global_state, |this, _, event, cx| {
            match event {
                GlobalEvent::WorkspaceSessionOpened(workspace_id, profile)
                    if profile.protocol == Protocol::Sftp =>
                {
                    this.connect_projection(workspace_id.clone(), profile.clone());
                }
                GlobalEvent::SelectWorkspaceSession(workspace_id) => {
                    if this.selected_workspace_id == *workspace_id {
                        return;
                    }
                    if let Some(previous_workspace_id) = this.selected_workspace_id.as_deref() {
                        this.local_restore_requests.remove(previous_workspace_id);
                    }
                    this.selected_workspace_id = workspace_id.clone();
                    this.remote_list_state.reset_with_uniform_height(0, px(38.));
                    if let Some(workspace_id) = workspace_id
                        .as_deref()
                        .filter(|workspace_id| this.projections.contains_key(*workspace_id))
                    {
                        this.restore_local_path(workspace_id, cx);
                    }
                    this.sync_application_state();
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

    pub(in crate::gui::workspace) fn status_updates(&self) -> Arc<tokio::sync::Notify> {
        self.status_updates.clone()
    }

    pub(in crate::gui::workspace) fn connection_status(
        &self,
        workspace_id: &str,
    ) -> Option<TerminalStatus> {
        self.application.sftp_connection_status(workspace_id).ok()
    }
}

fn to_sftp_status(status: TerminalStatus) -> SftpStatus {
    match status {
        TerminalStatus::Connecting => SftpStatus::Connecting,
        TerminalStatus::Connected => SftpStatus::Connected,
        TerminalStatus::Disconnected => SftpStatus::Disconnected,
        TerminalStatus::Failed => SftpStatus::Failed,
    }
}

fn to_sftp_snapshot(summary: SftpDirectorySummary, status: SftpStatus) -> super::SftpSnapshot {
    super::SftpSnapshot {
        status,
        path: summary.path,
        entries: Arc::new(
            summary
                .entries
                .into_iter()
                .map(|entry| super::SftpEntry {
                    name: entry.name,
                    path: entry.path,
                    is_directory: entry.is_directory,
                    size: entry.size,
                    modified_at: entry
                        .modified_at
                        .and_then(|modified_at| u32::try_from(modified_at).ok()),
                })
                .collect(),
        ),
        loading: summary.loading,
        error: summary.error,
    }
}

fn to_local_snapshot(summary: SftpDirectorySummary) -> super::LocalSnapshot {
    super::LocalSnapshot {
        path: PathBuf::from(summary.path),
        entries: Arc::new(
            summary
                .entries
                .into_iter()
                .map(|entry| super::LocalEntry {
                    name: entry.name,
                    path: PathBuf::from(entry.path),
                    is_directory: entry.is_directory,
                    size: entry.size,
                    modified_at: entry.modified_at.and_then(|modified_at| {
                        UNIX_EPOCH.checked_add(Duration::from_secs(modified_at))
                    }),
                })
                .collect(),
        ),
        loading: summary.loading,
        error: summary.error,
    }
}

fn to_transfer_record(info: SftpTransferInfo) -> super::TransferRecord {
    super::TransferRecord {
        id: info.id,
        workspace_id: info.workspace_id,
        name: info.name,
        direction: info.direction,
        target: info.target,
        progress: info.progress,
        speed: info.speed_bytes_per_second,
        status: info.status,
    }
}
