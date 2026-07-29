use gpui::*;

use super::{PasteTerminal, SendTab, TerminalView};

impl TerminalView {
    pub(super) fn key_down(
        &mut self,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        let terminal_model = self.model(&workspace_id);
        let application_cursor = terminal_model
            .as_ref()
            .is_some_and(|model| model.read().frame.application_cursor);
        if let Some(bytes) = encode_keystroke(&event.keystroke, application_cursor) {
            self.send_input(&workspace_id, bytes);
            cx.stop_propagation();
        }
    }

    pub(super) fn send_tab(&mut self, _: &SendTab, _: &mut Window, cx: &mut Context<Self>) {
        self.send_action_input(b"\t", cx);
    }

    pub(super) fn paste_terminal(
        &mut self,
        _: &PasteTerminal,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        self.send_action_input(text.as_bytes(), cx);
        cx.stop_propagation();
    }

    fn send_action_input(&self, bytes: &[u8], cx: &App) {
        let Some(workspace_id) = self.selected_workspace_id.as_deref() else {
            return;
        };
        self.send_input(workspace_id, bytes.to_vec());
    }
}

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
        let byte = match key.as_bytes() {
            [letter] if letter.is_ascii_alphabetic() => letter.to_ascii_lowercase() & 0x1f,
            [b' '] | [b'@'] => 0,
            [b'['] => 0x1b,
            [b'\\'] => 0x1c,
            [b']'] => 0x1d,
            [b'^'] => 0x1e,
            [b'_'] => 0x1f,
            _ => return None,
        };
        return Some(vec![byte]);
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

fn encode_keystroke(keystroke: &Keystroke, application_cursor: bool) -> Option<Vec<u8>> {
    if let Some(sequence) = encode_special_key(&keystroke.key, application_cursor) {
        return Some(sequence.as_bytes().to_vec());
    }

    if keystroke.modifiers.control {
        let byte = match keystroke.key.as_bytes() {
            [letter] if letter.is_ascii_alphabetic() => letter.to_ascii_lowercase() & 0x1f,
            [b' '] | [b'@'] => 0,
            [b'['] => 0x1b,
            [b'\\'] => 0x1c,
            [b']'] => 0x1d,
            [b'^'] => 0x1e,
            [b'_'] => 0x1f,
            _ => return None,
        };
        return Some(vec![byte]);
    }

    let text = keystroke.key_char.as_ref()?;
    let mut bytes = Vec::with_capacity(text.len() + usize::from(keystroke.modifiers.alt));
    if keystroke.modifiers.alt {
        bytes.push(0x1b);
    }
    bytes.extend_from_slice(text.as_bytes());
    Some(bytes)
}

fn encode_special_key(key: &str, application_cursor: bool) -> Option<&'static str> {
    match key {
        "enter" => Some("\r"),
        "backspace" => Some("\x7f"),
        "tab" => Some("\t"),
        "escape" => Some("\x1b"),
        "up" => Some(if application_cursor {
            "\x1bOA"
        } else {
            "\x1b[A"
        }),
        "down" => Some(if application_cursor {
            "\x1bOB"
        } else {
            "\x1b[B"
        }),
        "right" => Some(if application_cursor {
            "\x1bOC"
        } else {
            "\x1b[C"
        }),
        "left" => Some(if application_cursor {
            "\x1bOD"
        } else {
            "\x1b[D"
        }),
        "home" => Some(if application_cursor {
            "\x1bOH"
        } else {
            "\x1b[H"
        }),
        "end" => Some(if application_cursor {
            "\x1bOF"
        } else {
            "\x1b[F"
        }),
        "delete" => Some("\x1b[3~"),
        "pageup" => Some("\x1b[5~"),
        "pagedown" => Some("\x1b[6~"),
        "insert" => Some("\x1b[2~"),
        "f1" => Some("\x1bOP"),
        "f2" => Some("\x1bOQ"),
        "f3" => Some("\x1bOR"),
        "f4" => Some("\x1bOS"),
        "f5" => Some("\x1b[15~"),
        "f6" => Some("\x1b[17~"),
        "f7" => Some("\x1b[18~"),
        "f8" => Some("\x1b[19~"),
        "f9" => Some("\x1b[20~"),
        "f10" => Some("\x1b[21~"),
        "f11" => Some("\x1b[23~"),
        "f12" => Some("\x1b[24~"),
        _ => None,
    }
}
