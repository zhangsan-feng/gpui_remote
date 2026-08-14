pub(in crate::gui::workspace::ssh) fn encode_control_key(key: &str) -> Option<u8> {
    match key.as_bytes() {
        [letter] if letter.is_ascii_alphabetic() => Some(letter.to_ascii_lowercase() & 0x1f),
        [b' '] | [b'@'] => Some(0),
        [b'['] => Some(0x1b),
        [b'\\'] => Some(0x1c),
        [b']'] => Some(0x1d),
        [b'^'] => Some(0x1e),
        [b'_'] => Some(0x1f),
        _ => None,
    }
}

pub(in crate::gui::workspace::ssh) fn encode_special_key(
    key: &str,
    application_cursor: bool,
) -> Option<&'static str> {
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
