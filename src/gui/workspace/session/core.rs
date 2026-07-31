use gpui::Context;

use super::{OpenedWorkspaceSession, WorkspaceSession};
use crate::domain::session::SessionProfile;

impl WorkspaceSession {
    pub fn activate(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.selected_id.as_deref() == Some(id)
            || !self.sessions.iter().any(|item| item.id == id)
        {
            return;
        }
        self.selected_id = Some(id.to_owned());
        self.select_tab(id, cx);
        self.emit_selected_workspace(self.selected_id.clone(), cx);
        cx.notify();
    }

    pub fn close(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(index) = self.sessions.iter().position(|item| item.id == id) else {
            return;
        };
        self.sessions.remove(index);
        self.statuses.remove(id);
        let active_session_closed = self.selected_id.as_deref() == Some(id);
        if active_session_closed {
            self.selected_id = next_selected_session(&self.sessions, index);
        }
        self.rebuild_tabs(cx);

        self.emit_closed_workspace(id.to_owned(), cx);
        if active_session_closed {
            self.emit_selected_workspace(self.selected_id.clone(), cx);
        }
        cx.notify();
    }

    pub(in crate::gui::workspace) fn open(
        &mut self,
        workspace_id: String,
        profile: SessionProfile,
        cx: &mut Context<Self>,
    ) {
        self.sessions.push(OpenedWorkspaceSession {
            id: workspace_id.clone(),
            profile,
        });
        self.selected_id = Some(workspace_id.clone());
        self.rebuild_tabs(cx);
        self.emit_selected_workspace(Some(workspace_id), cx);
        cx.notify();
    }
}

fn next_selected_session(
    sessions: &[OpenedWorkspaceSession],
    removed_index: usize,
) -> Option<String> {
    sessions
        .get(removed_index.min(sessions.len().saturating_sub(1)))
        .map(|item| item.id.clone())
}
