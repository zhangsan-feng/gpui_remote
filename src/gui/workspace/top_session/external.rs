use std::collections::HashMap;

use gpui_kit::Context;

use crate::{
    data_context::DataSnapshot,
    global_state::{GlobalEvent, read_global_state},
};

use super::{OpenedWorkspaceSession, WorkspaceSession};

impl WorkspaceSession {
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
        cx: &mut Context<Self>,
    ) {
        read_global_state(cx).update(cx, |_, cx| {
            cx.emit(GlobalEvent::SelectWorkspaceSession(selected_id));
        });
    }

    pub(super) fn emit_closed_workspace(&self, workspace_id: String, cx: &mut Context<Self>) {
        read_global_state(cx).update(cx, |_, cx| {
            cx.emit(GlobalEvent::CloseWorkspaceSession { workspace_id });
        });
    }
}
