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
        self.observed_gui_snapshot_version = None;
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
            let Ok(gui_snapshot_version) = application.terminal_gui_snapshot_version(&workspace_id)
            else {
                continue;
            };
            let Some(model) = self.models.get(&workspace_id) else {
                continue;
            };
            let previous_gui_snapshot_version = model.gui_snapshot_version();
            if previous_gui_snapshot_version != gui_snapshot_version {
                let Ok(mcp_snapshot_version) =
                    application.terminal_mcp_snapshot_version(&workspace_id)
                else {
                    continue;
                };
                if let Ok(data) = application.terminal_snapshot(&workspace_id) {
                    let previous_mcp_snapshot_version = model.mcp_snapshot_version();
                    if previous_mcp_snapshot_version != mcp_snapshot_version {
                        log::debug!(
                            "SSH GUI terminal projection refreshed: workspace_id={workspace_id}, from_mcp_snapshot_version={previous_mcp_snapshot_version}, to_mcp_snapshot_version={mcp_snapshot_version}"
                        );
                    }
                    model.replace(data, mcp_snapshot_version, gui_snapshot_version);
                }
            }
        }
        let current_gui_snapshot_version =
            self.selected_workspace_id
                .as_deref()
                .and_then(|workspace_id| {
                    self.model(workspace_id)
                        .map(|model| (workspace_id.to_owned(), model.gui_snapshot_version()))
                });
        if current_gui_snapshot_version != self.observed_gui_snapshot_version {
            self.observed_gui_snapshot_version = current_gui_snapshot_version;
            cx.notify();
        }
    }
}
