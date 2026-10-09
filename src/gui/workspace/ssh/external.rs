mod api {
    use std::sync::Arc;

    use super::super::{TerminalView, core::TerminalModel};

    impl TerminalView {
        pub(in crate::gui::workspace) fn model(
            &self,
            workspace_id: &str,
        ) -> Option<Arc<TerminalModel>> {
            Some(self.models.get(workspace_id)?.clone())
        }
    }
}

mod lifecycle {
    use gpui_kit::Context;

    use std::sync::Arc;

    use crate::data_context::{DataSnapshot, WorkspaceSummary};
    use crate::domain::{
        session::Protocol,
        terminal::{TerminalData, TerminalFrame, TerminalStatus},
    };

    use super::super::TerminalView;

    impl TerminalView {
        pub(in crate::gui::workspace) fn apply_snapshot(
            &mut self,
            snapshot: &DataSnapshot,
            cx: &mut Context<Self>,
        ) {
            let existing = self.models.keys().cloned().collect::<Vec<_>>();
            for workspace_id in existing {
                if !snapshot.workspaces.iter().any(|workspace| {
                    workspace.workspace_id == workspace_id && workspace.protocol == Protocol::Ssh
                }) {
                    self.close_projection(&workspace_id);
                }
            }
            for workspace in snapshot
                .workspaces
                .iter()
                .filter(|workspace| workspace.protocol == Protocol::Ssh)
            {
                self.initialize_projection(workspace.workspace_id.clone(), workspace.clone());
            }
            let selected = snapshot
                .selected_workspace_id
                .as_ref()
                .filter(|workspace_id| self.models.contains_key(workspace_id.as_str()));
            self.set_selected_workspace(selected.cloned(), cx);
            self.notify_if_model_changed(cx);
        }

        pub(in crate::gui::workspace::ssh) fn initialize_projection(
            &mut self,
            workspace_id: String,
            profile: WorkspaceSummary,
        ) {
            if self.models.contains_key(&workspace_id) {
                return;
            }
            self.focus_pending = true;
            self.models.insert(
                workspace_id,
                Arc::new(super::super::core::TerminalModel::new(TerminalData {
                    frame: Arc::new(TerminalFrame::default()),
                    status: TerminalStatus::Connecting,
                    message: Some(format!("正在建立 {} 连接…", profile.protocol)),
                })),
            );
        }

        pub(in crate::gui::workspace::ssh) fn close_projection(&mut self, workspace_id: &str) {
            self.models.remove(workspace_id);
        }
    }
}
