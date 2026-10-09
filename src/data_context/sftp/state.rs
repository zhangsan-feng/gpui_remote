use crate::domain::session::Protocol;

use super::super::DataContext;
use super::super::state::SftpWorkspaceData;

impl DataContext {
    pub(super) fn edit_sftp_state<R>(
        &self,
        workspace_id: &str,
        edit: impl FnOnce(&mut SftpWorkspaceData) -> (R, bool),
    ) -> Option<R> {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut snapshot = self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !snapshot.workspaces.iter().any(|workspace| {
            workspace.workspace_id == workspace_id && workspace.protocol == Protocol::Sftp
        }) {
            return None;
        }
        let mut sftp = self
            .sftp
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = sftp.get_mut(workspace_id)?;
        let (result, notify) = edit(state);
        let status = state.remote.status.clone();
        drop(sftp);
        let Some(index) = snapshot
            .workspaces
            .iter()
            .position(|workspace| workspace.workspace_id == workspace_id)
        else {
            return None;
        };
        snapshot.workspaces[index].status = status;
        if notify {
            snapshot.revision = snapshot.revision.wrapping_add(1);
            let revision = snapshot.revision;
            snapshot.workspaces[index].sftp_revision = revision;
            drop(snapshot);
            drop(_commit);
            self.notice.notify_gui_refresh();
        }
        Some(result)
    }
}
