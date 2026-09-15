use gpui_kit::*;

use crate::application::ApplicationContext;

use super::super::*;

impl TerminalView {
    pub(in crate::gui::workspace::ssh) fn set_selected_workspace(
        &mut self,
        workspace_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.selected_workspace_id == workspace_id {
            return;
        }
        self.selected_workspace_id = workspace_id;
        self.focus_pending = self.selected_workspace_id.is_some();
        self.reset_active_view();
        cx.notify();
    }

    pub(in crate::gui::workspace::ssh) fn reset_active_view(&mut self) {
        self.listed_workspace_id = None;
        self.last_pty_size = None;
        self.observed_update_revision = None;
        self.selection = None;
        self.selection_origin = None;
        self.selecting_text = false;
    }

    pub(in crate::gui::workspace::ssh) fn notify_if_model_changed(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        let workspace_ids = self.models.keys().cloned().collect::<Vec<_>>();
        for workspace_id in workspace_ids {
            let Ok(update_revision) = application.terminal_update_revision(&workspace_id) else {
                continue;
            };
            let Some(model) = self.models.get(&workspace_id) else {
                continue;
            };
            let previous_update_revision = model.update_revision();
            if previous_update_revision != update_revision {
                let Ok(revision) = application.terminal_revision(&workspace_id) else {
                    continue;
                };
                if let Ok(data) = application.terminal_snapshot(&workspace_id) {
                    let previous_revision = model.revision();
                    if previous_revision != revision {
                        log::debug!(
                            "SSH GUI terminal projection refreshed: workspace_id={workspace_id}, from_revision={previous_revision}, to_revision={revision}"
                        );
                    }
                    model.replace(data, revision, update_revision);
                }
            }
        }
        let current_update_revision =
            self.selected_workspace_id
                .as_deref()
                .and_then(|workspace_id| {
                    self.model(workspace_id)
                        .map(|model| (workspace_id.to_owned(), model.update_revision()))
                });
        if current_update_revision != self.observed_update_revision {
            self.observed_update_revision = current_update_revision;
            cx.notify();
        }
    }
}
