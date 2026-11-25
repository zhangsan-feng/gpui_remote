use crate::{
    domain::{host_info::HostInfoState, session::Protocol, terminal::TerminalStatus},
    event_bus::{EVENT_BUS, refresh_gui::GuiComponent},
};

use super::super::{DataContext, WorkspaceSummary};

impl DataContext {
    pub(crate) fn is_selected_workspace(&self, workspace_id: &str) -> bool {
        self.snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .selected_workspace_id
            .as_deref()
            == Some(workspace_id)
    }

    pub(crate) fn selected_ssh_host(&self) -> Option<(WorkspaceSummary, HostInfoState)> {
        let snapshot = self
            .snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let workspace_id = snapshot.selected_workspace_id.as_deref()?;
        let workspace = snapshot.workspaces.iter().find(|workspace| {
            workspace.workspace_id == workspace_id && workspace.protocol == Protocol::Ssh
        })?;
        let terminals = self
            .terminals
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Some((
            workspace.clone(),
            terminals.get(workspace_id)?.host_info.clone(),
        ))
    }

    pub(crate) fn publish_host_info(&self, workspace_id: &str, info: HostInfoState) {
        let mut terminals = self
            .terminals
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(terminal) = terminals.get_mut(workspace_id) else {
            return;
        };
        if terminal.snapshot.data.status != TerminalStatus::Connected || terminal.host_info == info
        {
            return;
        }
        terminal.host_info = info;
        drop(terminals);
        EVENT_BUS
            .refresh_gui
            .notify_workspace(&[GuiComponent::HostInfo], workspace_id);
    }
}
