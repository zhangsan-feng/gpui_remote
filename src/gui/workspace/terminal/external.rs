mod api {
    use std::sync::Arc;

    use tokio::sync::{Notify, mpsc};

    use crate::domain::terminal::TerminalSessionCommand;

    use super::super::{TerminalView, core::TerminalModel};

    impl TerminalView {
        pub(in crate::gui::workspace) fn model(
            &self,
            workspace_id: &str,
        ) -> Option<Arc<TerminalModel>> {
            Some(self.terminals.get(workspace_id)?.model.clone())
        }

        pub(in crate::gui::workspace) fn status_updates(&self) -> Arc<Notify> {
            self.status_updates.clone()
        }

        pub(in crate::gui::workspace) fn command_sender(
            &self,
            workspace_id: &str,
        ) -> Option<mpsc::UnboundedSender<TerminalSessionCommand>> {
            Some(self.terminals.get(workspace_id)?.commands.clone())
        }

        pub(in crate::gui::workspace) fn send_input(&self, workspace_id: &str, input: Vec<u8>) {
            if let Some(terminal) = self.terminals.get(workspace_id) {
                let _ = terminal.commands.send(TerminalSessionCommand::Input(input));
            }
        }
    }
}

mod key {
    use super::super::core::{encode_control_key, encode_special_key};

    pub(in crate::gui::workspace) fn encode_agent_key(
        key: &str,
        control: bool,
        alt: bool,
        shift: bool,
        application_cursor: bool,
    ) -> Option<Vec<u8>> {
        let normalized_key = key.to_ascii_lowercase();
        if let Some(sequence) = encode_special_key(&normalized_key, application_cursor) {
            let mut bytes = Vec::with_capacity(sequence.len() + usize::from(alt));
            if alt {
                bytes.push(0x1b);
            }
            bytes.extend_from_slice(sequence.as_bytes());
            return Some(bytes);
        }

        if control {
            return Some(vec![encode_control_key(key)?]);
        }

        let text = if shift && key.chars().count() == 1 {
            key.to_uppercase()
        } else {
            key.to_owned()
        };
        let mut bytes = Vec::with_capacity(text.len() + usize::from(alt));
        if alt {
            bytes.push(0x1b);
        }
        bytes.extend_from_slice(text.as_bytes());
        Some(bytes)
    }
}

mod lifecycle {
    use gpui::Context;

    use crate::global_state::{GlobalEvent, read_global_state};

    use super::super::{TerminalView, core::supports_terminal_protocol};

    impl TerminalView {
        pub(in crate::gui::workspace::terminal) fn start_subscribe(&self, cx: &mut Context<Self>) {
            let global_state = read_global_state(cx);
            cx.subscribe(&global_state, |this, _, event, cx| {
                match event {
                    GlobalEvent::OpenWorkspaceSession(workspace_id, profile) => {
                        if supports_terminal_protocol(&profile.protocol) {
                            this.connect(workspace_id.clone(), profile.clone());
                        }
                        return;
                    }
                    GlobalEvent::CloseWorkspaceSession { workspace_id } => this.close(workspace_id),
                    GlobalEvent::SelectWorkspaceSession(workspace_id) => {
                        this.set_selected_workspace(workspace_id.clone(), cx);
                        return;
                    }
                    _ => return,
                }
                this.reset_active_view();
                cx.notify();
            })
            .detach();
        }
    }
}

pub(in crate::gui::workspace) use key::encode_agent_key;
