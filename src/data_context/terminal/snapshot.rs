use crate::domain::{session::Protocol, terminal::McpTerminalHistoryPage};

use super::super::{DataContext, TerminalReadSnapshot};

impl DataContext {
    pub(crate) fn terminal_snapshot(&self, workspace_id: &str) -> Option<TerminalReadSnapshot> {
        let snapshot = self
            .snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !snapshot.workspaces.iter().any(|workspace| {
            workspace.workspace_id == workspace_id && workspace.protocol == Protocol::Ssh
        }) {
            return None;
        }
        self.terminals
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .map(|terminal| terminal.snapshot.clone())
    }

    pub(crate) fn terminal_history(
        &self,
        workspace_id: &str,
        offset: usize,
        limit: usize,
        since_mcp_snapshot_version: Option<u64>,
    ) -> Option<McpTerminalHistoryPage> {
        let snapshot = self
            .snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !snapshot.workspaces.iter().any(|workspace| {
            workspace.workspace_id == workspace_id && workspace.protocol == Protocol::Ssh
        }) {
            return None;
        }
        let terminals = self
            .terminals
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let terminal = terminals.get(workspace_id)?;
        let mut page = terminal.buffer.as_ref()?.read_text(offset, limit);
        page.mcp_snapshot_version = terminal.snapshot.mcp_snapshot_version;
        page.changed =
            since_mcp_snapshot_version.is_none_or(|version| version != page.mcp_snapshot_version);
        if !page.changed {
            page.text.clear();
            page.limit = 0;
            page.has_more = false;
        }
        Some(page)
    }
}
