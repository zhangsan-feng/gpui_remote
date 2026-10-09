use crate::domain::{
    session::Protocol,
    terminal::{TerminalData, TerminalFrame, TerminalStatus},
};

use super::super::state::{SftpWorkspaceData, TerminalWorkspaceData};
use super::super::{DataChange, DataContext, TerminalReadSnapshot, WorkspaceSummary};

impl DataContext {
    pub(crate) fn open_workspace(&self, profile: WorkspaceSummary) -> Result<(), String> {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let workspace_id = profile.workspace_id.clone();
        let protocol = profile.protocol;
        let mut snapshot = self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if snapshot
            .workspaces
            .iter()
            .any(|workspace| workspace.workspace_id == workspace_id)
        {
            return Err(format!("会话已打开: {workspace_id}"));
        }
        snapshot.workspaces.push(profile.clone());
        snapshot.selected_workspace_id = Some(workspace_id.clone());
        snapshot.revision = snapshot.revision.wrapping_add(1);
        match protocol {
            Protocol::Ssh => {
                self.terminals
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .insert(
                        workspace_id.clone(),
                        TerminalWorkspaceData {
                            snapshot: TerminalReadSnapshot {
                                data: std::sync::Arc::new(TerminalData {
                                    frame: std::sync::Arc::new(TerminalFrame::default()),
                                    status: TerminalStatus::Connecting,
                                    message: Some("正在建立 SSH 连接…".into()),
                                }),
                                mcp_snapshot_version: 0,
                                gui_snapshot_version: 0,
                            },
                            buffer: None,
                        },
                    );
            }
            Protocol::Sftp => {
                self.sftp
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .insert(workspace_id.clone(), SftpWorkspaceData::default());
            }
            _ => {}
        }
        drop(snapshot);
        drop(_commit);
        self.notice.notify_gui_refresh();
        self.notice
            .notify_session_lifecycle(DataChange::SessionOpened {
                workspace_id: workspace_id.clone(),
                profile,
            });
        self.notice
            .notify_session_lifecycle(DataChange::SessionSelected {
                workspace_id: Some(workspace_id),
            });
        Ok(())
    }

    pub(crate) fn close_workspace(&self, workspace_id: &str) -> Result<(), String> {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut snapshot = self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let index = snapshot
            .workspaces
            .iter()
            .position(|workspace| workspace.workspace_id == workspace_id)
            .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
        snapshot.workspaces.remove(index);
        let selected_after_close =
            if snapshot.selected_workspace_id.as_deref() == Some(workspace_id) {
                let selected = snapshot
                    .workspaces
                    .get(index.min(snapshot.workspaces.len().saturating_sub(1)))
                    .map(|workspace| workspace.workspace_id.clone());
                snapshot.selected_workspace_id = selected.clone();
                Some(selected)
            } else {
                None
            };
        snapshot.revision = snapshot.revision.wrapping_add(1);
        self.terminals
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id);
        self.sftp
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id);
        drop(snapshot);
        drop(_commit);
        self.notice.notify_gui_refresh();
        self.notice
            .notify_session_lifecycle(DataChange::SessionClosed {
                workspace_id: workspace_id.to_owned(),
            });
        if let Some(selected) = selected_after_close {
            self.notice
                .notify_session_lifecycle(DataChange::SessionSelected {
                    workspace_id: selected,
                });
        }
        Ok(())
    }

    pub(crate) fn select_workspace(&self, workspace_id: Option<String>) -> Result<(), String> {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(id) = workspace_id.as_deref() {
                if !snapshot
                    .workspaces
                    .iter()
                    .any(|workspace| workspace.workspace_id == id)
                {
                    return Err(format!("会话不存在: {id}"));
                }
            }
            if snapshot.selected_workspace_id == workspace_id {
                return Ok(());
            }
            snapshot.selected_workspace_id = workspace_id.clone();
            snapshot.revision = snapshot.revision.wrapping_add(1);
        }
        drop(_commit);
        self.notice.notify_gui_refresh();
        self.notice
            .notify_session_lifecycle(DataChange::SessionSelected { workspace_id });
        Ok(())
    }

    pub(crate) fn update_database_workspace(
        &self,
        workspace_id: &str,
        status: TerminalStatus,
    ) -> bool {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(index) = snapshot.workspaces.iter().position(|workspace| {
                workspace.workspace_id == workspace_id
                    && matches!(
                        workspace.protocol,
                        Protocol::Mysql | Protocol::Pgsql | Protocol::Redis
                    )
            }) else {
                return false;
            };
            snapshot.revision = snapshot.revision.wrapping_add(1);
            let revision = snapshot.revision;
            let workspace = &mut snapshot.workspaces[index];
            workspace.status = status;
            workspace.database_revision = revision;
        }
        drop(_commit);
        self.notice.notify_gui_refresh();
        true
    }
}
