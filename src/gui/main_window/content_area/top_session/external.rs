use std::collections::HashMap;

use gpui_kit::Context;

use crate::{
    data_context::DataSnapshot,
    event_bus::{EVENT_BUS, gui_event::GuiEvent},
};

use super::{OpenedWorkspaceSession, WorkspaceSession};

impl WorkspaceSession {
    pub(in crate::gui::main_window::content_area) fn refresh_session_from_context(
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
        if session.profile == profile {
            return;
        }
        self.statuses
            .insert(workspace_id.to_owned(), profile.status.clone());
        session.profile = profile;
        cx.notify();
    }

    pub(in crate::gui::main_window::content_area) fn apply_snapshot(
        &mut self,
        snapshot: &DataSnapshot,
        cx: &mut Context<Self>,
    ) {
        let mut sessions = snapshot
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
        // Preserve the user's tab order across snapshot refreshes.
        sessions.sort_by_key(|session| {
            self.sessions
                .iter()
                .position(|old| old.id == session.id)
                .unwrap_or(usize::MAX)
        });
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
        let selected_changed = self.selected_id != snapshot.selected_workspace_id;
        self.sessions = sessions;
        self.selected_id = snapshot.selected_workspace_id.clone();
        if selected_changed {
            if let Some(index) = self
                .sessions
                .iter()
                .position(|session| Some(&session.id) == self.selected_id.as_ref())
            {
                self.tabs_scroll.scroll_to_item(index);
            }
        }
        self.statuses = statuses;
        cx.notify();
    }

    pub(in crate::gui::main_window::content_area) fn sessions(&self) -> &[OpenedWorkspaceSession] {
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
