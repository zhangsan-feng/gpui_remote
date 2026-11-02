use std::collections::HashMap;

use gpui_kit::Context;

use crate::{
    data_context::DataSnapshot,
    event_bus::{EVENT_BUS, gui_event::GuiEvent},
};

use super::{OpenedWorkspaceSession, WorkspaceSession};

impl WorkspaceSession {
    pub(in crate::gui::workspace) fn refresh_session_from_context(
        &mut self,
        workspace_id: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(profile) = crate::data_context::DATA_CONTEXT.workspace_summary(workspace_id)
        else {
            return;
        };
        let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == workspace_id)
        else {
            return;
        };
        if self.statuses.get(workspace_id) == Some(&profile.status) {
            return;
        }
        self.statuses
            .insert(workspace_id.to_owned(), profile.status.clone());
        session.profile = profile;
        self.refresh_tab(workspace_id, cx);
        cx.notify();
    }

    pub(in crate::gui::workspace) fn apply_snapshot(
        &mut self,
        snapshot: &DataSnapshot,
        cx: &mut Context<Self>,
    ) {
        let sessions = snapshot
            .workspaces
            .iter()
            .map(|workspace| OpenedWorkspaceSession {
                id: workspace.workspace_id.clone(),
                profile: workspace.clone(),
            })
            .collect::<Vec<_>>();
        let statuses = snapshot
            .workspaces
            .iter()
            .map(|workspace| (workspace.workspace_id.clone(), workspace.status.clone()))
            .collect::<HashMap<_, _>>();
        let same_sessions = self.sessions.len() == sessions.len()
            && self.sessions.iter().zip(&sessions).all(|(old, new)| {
                old.id == new.id
                    && old.profile.profile_id == new.profile.profile_id
                    && old.profile.title == new.profile.title
                    && old.profile.host == new.profile.host
                    && old.profile.protocol == new.profile.protocol
                    && old.profile.is_ssh_tunnel == new.profile.is_ssh_tunnel
            });
        if same_sessions
            && self.selected_id == snapshot.selected_workspace_id
            && self.statuses == statuses
        {
            return;
        }
        self.sessions = sessions;
        self.selected_id = snapshot.selected_workspace_id.clone();
        self.statuses = statuses;
        self.rebuild_tabs(cx);
        cx.notify();
    }

    pub(in crate::gui::workspace) fn sessions(&self) -> &[OpenedWorkspaceSession] {
        &self.sessions
    }

    pub(super) fn emit_selected_workspace(
        &self,
        selected_id: Option<String>,
        _cx: &mut Context<Self>,
    ) {
        EVENT_BUS.gui_event.send(
            self.gui_scope,
            GuiEvent::SelectWorkspaceSession(selected_id),
        );
    }

    pub(super) fn emit_closed_workspace(&self, workspace_id: String, _cx: &mut Context<Self>) {
        EVENT_BUS.gui_event.send(
            self.gui_scope,
            GuiEvent::CloseWorkspaceSession { workspace_id },
        );
    }
}
