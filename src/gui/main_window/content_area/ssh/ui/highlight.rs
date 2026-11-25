use std::ops::Range;

use crate::domain::terminal::TerminalFrame;

// Use the terminal theme's ANSI palette for local highlights as well.
#[derive(Clone, Copy)]
pub(super) enum LocalColor {
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    Gray = 8,
}

#[derive(Clone, Copy)]
pub(super) struct Highlight {
    pub start: usize,
    pub end: usize,
    pub color: LocalColor,
}

impl Highlight {
    fn new(range: Range<usize>, color: LocalColor) -> Self {
        Self {
            start: range.start,
            end: range.end,
            color,
        }
    }
}

pub(super) fn frame_highlights(frame: &TerminalFrame) -> Vec<Vec<Highlight>> {
    if frame.alternate_screen {
        return Vec::new();
    }

    let mut listing = false;
    let mut classify = false;
    let mut continuation = false;
    frame
        .lines
        .iter()
        .map(|row| {
            let text: String = row.spans.iter().map(|span| span.text.as_str()).collect();
            let text = text.trim_end();
            let starts_line = !continuation;
            continuation = row.wrapped;
            if starts_line && let Some((host_end, prompt_end)) = shell_prompt(text) {
                let command = text[prompt_end..].trim_start();
                listing = !row.wrapped && is_list_command(command);
                classify = listing
                    && command
                        .split_whitespace()
                        .skip(1)
                        .take_while(|argument| *argument != "--")
                        .any(|argument| {
                            argument == "--classify"
                                || argument == "--classify=always"
                                || argument == "--classify=auto"
                                || argument.strip_prefix('-').is_some_and(|flags| {
                                    !flags.starts_with('-') && flags.contains('F')
                                })
                        });
                let mut highlights = vec![
                    Highlight::new(0..host_end, LocalColor::Green),
                    Highlight::new(host_end..prompt_end, LocalColor::Blue),
                ];
                if let Some(program) = command.split_whitespace().next() {
                    let start = text.len() - command.len();
                    highlights.push(Highlight::new(
                        start..start + program.len(),
                        LocalColor::Cyan,
                    ));
                }
                return highlights;
            }

            if starts_line && let Some(highlight) = status_highlight(text) {
                return vec![highlight];
            }
            if starts_line && let Some(highlights) = long_listing(text) {
                return highlights;
            }
            if listing && !text.starts_with("total ") && !text.starts_with("总用量") {
                return short_listing(text, classify);
            }
            Vec::new()
        })
        .collect()
}

fn shell_prompt(text: &str) -> Option<(usize, usize)> {
    let (user, rest) = text.split_once('@')?;
    let (host, path_and_command) = rest.split_once(':')?;
    let identifier = |value: &str| {
        !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    };
    if !identifier(user) || !identifier(host) || !path_and_command.starts_with(['/', '~', '.']) {
        return None;
    }
    let (marker, _) = path_and_command
        .char_indices()
        .find(|&(index, character)| {
            matches!(character, '$' | '#' | '%')
                && path_and_command[index + 1..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace)
        })?;
    let host_end = user.len() + 1 + host.len();
    Some((host_end, host_end + 1 + marker + 1))
}

fn is_list_command(command: &str) -> bool {
    !command.contains([';', '|', '&', '<', '>'])
        && command
            .split_whitespace()
            .next()
            .is_some_and(|program| matches!(program.rsplit('/').next(), Some("ls")))
}

fn status_highlight(text: &str) -> Option<Highlight> {
    let trimmed = text.trim_start();
    let start = text.len() - trimmed.len();
    let label = trimmed.split_whitespace().next()?.trim_end_matches(':');
    let color = match label.trim_start_matches('[').trim_end_matches(']') {
        "ERROR" | "FATAL" | "FAIL" | "FAILED" | "ls"
            if label != "ls" || trimmed.starts_with("ls:") =>
        {
            LocalColor::Red
        }
        "WARN" | "WARNING" => LocalColor::Yellow,
        "OK" | "PASS" | "SUCCESS" => LocalColor::Green,
        "INFO" | "DEBUG" => LocalColor::Cyan,
        _ => return None,
    };
    Some(Highlight::new(start..start + label.len(), color))
}

fn long_listing(text: &str) -> Option<Vec<Highlight>> {
    let mut fields = text.split_whitespace();
    let permissions = fields.next()?;
    let mode = permissions.as_bytes().get(..10)?;
    if !b"-dlcbps".contains(&mode[0])
        || !mode[1..].iter().all(|byte| b"rwxstST-".contains(byte))
        || !matches!(permissions.get(10..)?, "" | "+" | "." | "@")
        || fields.next()?.parse::<u64>().is_err()
    {
        return None;
    }
    let _owner = fields.next()?;
    let _group = fields.next()?;
    let size = fields.next()?;
    let date = fields.next()?;
    let second = fields.next()?;
    let last = if date.as_bytes().get(4) == Some(&b'-') && second.contains(':') {
        second
    } else {
        fields.next()?
    };
    if !size.starts_with(|character: char| character.is_ascii_digit())
        || !(last.contains(':') || last.parse::<u32>().is_ok())
    {
        return None;
    }

    // These tokens borrow from the original row, so their offsets include padding.
    let mode_start = permissions.as_ptr() as usize - text.as_ptr() as usize;
    let date_start = date.as_ptr() as usize - text.as_ptr() as usize;
    let last_start = last.as_ptr() as usize - text.as_ptr() as usize;
    let time_end = last_start + last.len();
    let entry = text[time_end..].trim_start();
    if entry.is_empty() {
        return None;
    }
    let name_start = text.len() - entry.len();
    let name_color = match mode[0] {
        b'd' => LocalColor::Blue,
        b'l' => LocalColor::Cyan,
        b'b' | b'c' | b'p' | b's' => LocalColor::Yellow,
        _ if mode[1..].iter().any(|byte| b"xst".contains(byte)) => LocalColor::Green,
        _ => file_color(entry),
    };
    let mut highlights = Vec::with_capacity(permissions.len() + 4);
    highlights.push(Highlight::new(mode_start..mode_start + 1, name_color));
    for (index, permission) in permissions.bytes().enumerate().skip(1) {
        let color = match permission {
            b'r' => LocalColor::Cyan,
            b'w' => LocalColor::Yellow,
            b'x' => LocalColor::Magenta,
            b'-' => LocalColor::Gray,
            _ => LocalColor::Red,
        };
        let start = mode_start + index;
        highlights.push(Highlight::new(start..start + 1, color));
    }
    if last.contains(':') {
        let date_end = text[..last_start].trim_end().len();
        highlights.push(Highlight::new(date_start..date_end, LocalColor::Cyan));
        highlights.push(Highlight::new(last_start..time_end, LocalColor::Green));
    } else {
        highlights.push(Highlight::new(date_start..time_end, LocalColor::Cyan));
    }
    if mode[0] == b'l'
        && let Some((name, target)) = entry.split_once(" -> ")
    {
        highlights.push(Highlight::new(
            name_start..name_start + name.len(),
            name_color,
        ));
        highlights.push(Highlight::new(
            text.len() - target.len()..text.len(),
            file_color(target),
        ));
    } else {
        highlights.push(Highlight::new(name_start..text.len(), name_color));
    }
    Some(highlights)
}

fn short_listing(text: &str, classify: bool) -> Vec<Highlight> {
    let mut offset = 0;
    text.split("  ")
        .filter_map(|column| {
            let entry = column.trim();
            let start = offset + column.len() - column.trim_start().len();
            offset += column.len() + 2;
            let color = if entry.ends_with('/') {
                LocalColor::Blue
            } else if classify && entry.ends_with('*') {
                LocalColor::Green
            } else if classify && entry.ends_with('@') {
                LocalColor::Cyan
            } else {
                file_color(entry)
            };
            (!entry.is_empty()).then(|| Highlight::new(start..start + entry.len(), color))
        })
        .collect()
}

fn file_color(entry: &str) -> LocalColor {
    match entry.rsplit_once('.').map(|(_, extension)| extension) {
        Some("zip" | "gz" | "xz" | "tar" | "7z" | "rar" | "bz2" | "zst") => LocalColor::Magenta,
        Some("log" | "conf" | "ini" | "json" | "toml" | "yaml" | "yml") => LocalColor::Yellow,
        _ => LocalColor::Cyan,
    }
}
