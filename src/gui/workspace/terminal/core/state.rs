use gpui::*;

use super::super::*;

impl TerminalView {
    pub(in crate::gui::workspace::terminal) fn set_selected_workspace(
        &mut self,
        workspace_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.selected_workspace_id == workspace_id {
            return;
        }
        self.selected_workspace_id = workspace_id;
        self.reset_active_view();
        cx.notify();
    }

    pub(in crate::gui::workspace::terminal) fn reset_active_view(&mut self) {
        self.listed_workspace_id = None;
        self.last_pty_size = None;
        self.observed_revision = None;
        self.selection = None;
        self.selecting_text = false;
    }

    pub(in crate::gui::workspace::terminal) fn notify_if_model_changed(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let current_revision = self
            .selected_workspace_id
            .as_deref()
            .and_then(|workspace_id| {
                self.model(workspace_id)
                    .map(|model| (workspace_id.to_owned(), model.revision()))
            });
        if current_revision != self.observed_revision {
            self.observed_revision = current_revision;
            cx.notify();
        }
    }
}
