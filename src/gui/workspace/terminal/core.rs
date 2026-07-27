use gpui::*;

use super::*;

impl TerminalView {
    pub(super) fn reset_active_view(&mut self) {
        self.listed_workspace_id = None;
        self.last_pty_size = None;
        self.observed_revision = None;
        self.selection = None;
        self.selecting_text = false;
    }

    pub(super) fn notify_if_model_changed(&mut self, cx: &mut Context<Self>) {
        let current_revision = {
            let workspace = self.workspace.read(cx);
            workspace.active_session_id().and_then(|workspace_id| {
                self.model(workspace_id)
                    .map(|model| (workspace_id.to_owned(), model.revision()))
            })
        };
        if current_revision != self.observed_revision {
            self.observed_revision = current_revision;
            cx.notify();
        }
    }
}
