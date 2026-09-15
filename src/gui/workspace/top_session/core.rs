use gpui_kit::Context;

use super::{OpenedWorkspaceSession, WorkspaceSession};
use crate::domain::session::SessionProfile;

impl WorkspaceSession {
    pub fn activate(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.selected_id.as_deref() == Some(id)
            || !self.sessions.iter().any(|item| item.id == id)
        {
            return;
        }
        self.emit_selected_workspace(Some(id.to_owned()), cx);
    }

    pub fn close(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.sessions.iter().any(|item| item.id == id) {
            return;
        }
        self.emit_closed_workspace(id.to_owned(), cx);
    }

    pub(in crate::gui::workspace) fn open(
        &mut self,
        workspace_id: String,
        profile: SessionProfile,
        cx: &mut Context<Self>,
    ) {
        log::debug!(
            "GUI workspace tab opened: workspace_id={workspace_id}, profile_id={}, protocol={}",
            profile.id,
            profile.protocol
        );
        if self
            .sessions
            .iter()
            .any(|session| session.id == workspace_id)
        {
            return;
        }
        self.sessions.push(OpenedWorkspaceSession {
            id: workspace_id,
            profile,
        });
        self.rebuild_tabs(cx);
        cx.notify();
    }

    pub(super) fn select_from_application(
        &mut self,
        selected_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let selected_id = selected_id.filter(|id| {
            self.sessions
                .iter()
                .any(|opened_session| opened_session.id.as_str() == id)
        });
        if self.selected_id == selected_id {
            if let Some(selected_id) = selected_id.as_deref() {
                self.select_tab(selected_id, cx);
            }
            return;
        }
        self.selected_id = selected_id.clone();
        if let Some(selected_id) = selected_id.as_deref() {
            self.select_tab(selected_id, cx);
        }
        cx.notify();
    }

    pub(super) fn close_from_application(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(index) = self.sessions.iter().position(|item| item.id == id) else {
            return;
        };
        self.sessions.remove(index);
        self.statuses.remove(id);
        if self.selected_id.as_deref() == Some(id) {
            self.selected_id = None;
        }
        self.rebuild_tabs(cx);
        cx.notify();
    }
}
