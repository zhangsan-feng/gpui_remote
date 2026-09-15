mod api {
    use std::sync::Arc;

    use tokio::sync::Notify;

    use super::super::{TerminalView, core::TerminalModel};

    impl TerminalView {
        pub(in crate::gui::workspace) fn model(
            &self,
            workspace_id: &str,
        ) -> Option<Arc<TerminalModel>> {
            Some(self.models.get(workspace_id)?.clone())
        }

        pub(in crate::gui::workspace) fn status_updates(&self) -> Arc<Notify> {
            self.status_updates.clone()
        }
    }
}

mod lifecycle {
    use gpui_kit::Context;

    use crate::global_state::{GlobalEvent, read_global_state};

    use std::sync::Arc;

    use crate::domain::{
        session::SessionProfile,
        terminal::{TerminalData, TerminalFrame, TerminalStatus},
    };

    use super::super::TerminalView;

    impl TerminalView {
        pub(in crate::gui::workspace::ssh) fn start_subscribe(&self, cx: &mut Context<Self>) {
            let global_state = read_global_state(cx);
            cx.subscribe(&global_state, |this, _, event, cx| {
                match event {
                    GlobalEvent::WorkspaceSessionOpened(workspace_id, profile) => {
                        if profile.protocol == crate::domain::session::Protocol::Ssh {
                            log::debug!(
                                "SSH GUI projection event received: workspace_id={workspace_id}, profile_id={}, host={}",
                                profile.id,
                                profile.host
                            );
                            this.initialize_projection(workspace_id.clone(), profile.clone());
                            this.notify_if_model_changed(cx);
                        }
                    }
                    GlobalEvent::WorkspaceSessionClosed { workspace_id } => {
                        this.close_projection(workspace_id);
                        this.reset_active_view();
                    }
                    GlobalEvent::WorkspaceSessionSelected(workspace_id) => {
                        this.set_selected_workspace(workspace_id.clone(), cx);
                        this.notify_if_model_changed(cx);
                    }
                    _ => return,
                }
                this.reset_active_view();
                cx.notify();
            })
            .detach();
        }

        pub(in crate::gui::workspace::ssh) fn initialize_projection(
            &mut self,
            workspace_id: String,
            profile: SessionProfile,
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
