use gpui_kit::Context;

use super::WorkspaceSession;

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
}
