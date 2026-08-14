use std::collections::HashMap;

use gpui::Context;

use crate::{
    domain::terminal::TerminalStatus,
    global_state::{GlobalEvent, read_global_state},
};

use super::{OpenedWorkspaceSession, WorkspaceSession};

impl WorkspaceSession {
    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        cx.subscribe(&global_state, |this, _, event, cx| match event {
            GlobalEvent::OpenWorkspaceSession(workspace_id, profile) => {
                this.open(workspace_id.clone(), profile.clone(), cx);
            }
            _ => {}
        })
        .detach();
    }

    pub(in crate::gui::workspace) fn sessions(&self) -> &[OpenedWorkspaceSession] {
        &self.sessions
    }

    pub(in crate::gui::workspace) fn selected_id(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    pub(in crate::gui::workspace) fn update_statuses(
        &mut self,
        statuses: HashMap<String, TerminalStatus>,
        cx: &mut Context<Self>,
    ) {
        if self.statuses != statuses {
            self.statuses = statuses;
            self.rebuild_tabs(cx);
        }
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

pub(in crate::gui::workspace) fn terminal_statuses(
    sessions: &[OpenedWorkspaceSession],
    terminal_status: impl Fn(&str) -> Option<TerminalStatus>,
) -> HashMap<String, TerminalStatus> {
    sessions
        .iter()
        .map(|opened_session| {
            let id = opened_session.id.clone();
            let status = terminal_status(&id).unwrap_or(TerminalStatus::Connecting);
            (id, status)
        })
        .collect()
}
