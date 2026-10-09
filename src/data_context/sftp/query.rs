use crate::domain::session::Protocol;

use super::super::{DataContext, SftpWorkspaceSnapshot};

impl DataContext {
    pub(crate) fn sftp_workspace_snapshot(
        &self,
        workspace_id: &str,
    ) -> Option<SftpWorkspaceSnapshot> {
        let snapshot = self
            .snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !snapshot.workspaces.iter().any(|workspace| {
            workspace.workspace_id == workspace_id && workspace.protocol == Protocol::Sftp
        }) {
            return None;
        }
        self.sftp
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .map(super::snapshot::to_workspace_snapshot)
    }
}
