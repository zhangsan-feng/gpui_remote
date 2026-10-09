use std::sync::Arc;

use crate::domain::terminal::{TerminalData, TerminalStatus};

use super::super::DataContext;
use super::TerminalBuffer;

impl DataContext {
    pub(crate) fn publish_terminal(
        &self,
        workspace_id: &str,
        data: TerminalData,
        content_changed: bool,
    ) -> bool {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut snapshot = self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !snapshot.workspaces.iter().any(|item| {
            item.workspace_id == workspace_id
                && item.protocol == crate::domain::session::Protocol::Ssh
        }) {
            return false;
        }
        let mut terminals = self
            .terminals
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(terminal) = terminals.get_mut(workspace_id) else {
            return false;
        };
        let status = data.status.clone();
        terminal.snapshot.data = Arc::new(data);
        if content_changed {
            terminal.snapshot.mcp_snapshot_version =
                terminal.snapshot.mcp_snapshot_version.wrapping_add(1);
        }
        terminal.snapshot.gui_snapshot_version =
            terminal.snapshot.gui_snapshot_version.wrapping_add(1);
        drop(terminals);
        snapshot.revision = snapshot.revision.wrapping_add(1);
        let revision = snapshot.revision;
        let workspace = snapshot
            .workspaces
            .iter_mut()
            .find(|item| item.workspace_id == workspace_id)
            .unwrap();
        workspace.status = status;
        workspace.terminal_revision = revision;
        drop(snapshot);
        drop(_commit);
        self.notice.notify_gui_refresh();
        true
    }

    pub(crate) fn initialize_terminal_buffer(&self, workspace_id: &str) -> bool {
        let snapshot = self
            .snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !snapshot.workspaces.iter().any(|workspace| {
            workspace.workspace_id == workspace_id
                && workspace.protocol == crate::domain::session::Protocol::Ssh
        }) {
            return false;
        }
        let mut terminals = self
            .terminals
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(terminal) = terminals.get_mut(workspace_id) else {
            return false;
        };
        terminal.buffer = Some(TerminalBuffer::new());
        true
    }

    pub(crate) fn with_terminal_buffer<R>(
        &self,
        workspace_id: &str,
        update: impl FnOnce(&mut TerminalBuffer) -> R,
    ) -> Option<R> {
        let mut terminals = self
            .terminals
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        terminals
            .get_mut(workspace_id)
            .and_then(|terminal| terminal.buffer.as_mut())
            .map(update)
    }

    pub(crate) fn update_terminal_status(
        &self,
        workspace_id: &str,
        status: TerminalStatus,
        message: Option<String>,
    ) -> bool {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut snapshot = self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !snapshot.workspaces.iter().any(|workspace| {
            workspace.workspace_id == workspace_id
                && workspace.protocol == crate::domain::session::Protocol::Ssh
        }) {
            return false;
        }
        let mut terminals = self
            .terminals
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(terminal) = terminals.get_mut(workspace_id) else {
            return false;
        };
        let mut data = terminal.snapshot.data.as_ref().clone();
        data.status = status.clone();
        data.message = message;
        terminal.snapshot.data = Arc::new(data);
        terminal.snapshot.gui_snapshot_version =
            terminal.snapshot.gui_snapshot_version.wrapping_add(1);
        drop(terminals);
        snapshot.revision = snapshot.revision.wrapping_add(1);
        let revision = snapshot.revision;
        let workspace = snapshot
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.workspace_id == workspace_id)
            .unwrap();
        workspace.status = status;
        workspace.terminal_revision = revision;
        drop(snapshot);
        drop(_commit);
        self.notice.notify_gui_refresh();
        true
    }
}
