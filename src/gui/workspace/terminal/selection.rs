use gpui::*;
use unicode_width::UnicodeWidthChar;

use crate::domain::terminal::{TerminalFrame, TerminalLine, TerminalStyle};

use super::{CopyTerminal, TerminalView};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TerminalPoint {
    pub(super) row: usize,
    pub(super) column: usize,
}

#[derive(Clone)]
pub(super) struct TerminalSelection {
    pub(super) workspace_id: String,
    anchor: TerminalPoint,
    head: TerminalPoint,
}

pub(super) struct SelectedFragment {
    pub(super) text: String,
    pub(super) style: TerminalStyle,
    pub(super) selected: bool,
}

impl TerminalSelection {
    fn range(&self) -> (TerminalPoint, TerminalPoint) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    fn contains(&self, point: TerminalPoint) -> bool {
        let (start, end) = self.range();
        start <= point && point <= end
    }
}

impl TerminalView {
    pub(super) fn copy_terminal(
        &mut self,
        _: &CopyTerminal,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(selection) = &self.selection else {
            return;
        };
        let Some(model) = self.model(&selection.workspace_id) else {
            return;
        };
        let frame = model.read().frame.clone();
        let text = selected_text(&frame, selection);
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        cx.stop_propagation();
    }

    pub(super) fn select_point(
        &mut self,
        workspace_id: String,
        point: TerminalPoint,
        extend: bool,
        cx: &mut Context<Self>,
    ) {
        self.selecting_text = true;
        if extend {
            if let Some(selection) = self
                .selection
                .as_mut()
                .filter(|selection| selection.workspace_id == workspace_id)
            {
                selection.head = point;
                cx.notify();
                return;
            }
        }
        self.selection = Some(TerminalSelection {
            workspace_id,
            anchor: point,
            head: point,
        });
        cx.notify();
    }

    pub(super) fn extend_selection(
        &mut self,
        workspace_id: &str,
        point: TerminalPoint,
        cx: &mut Context<Self>,
    ) {
        if !self.selecting_text {
            return;
        }
        if let Some(selection) = self
            .selection
            .as_mut()
            .filter(|selection| selection.workspace_id == workspace_id)
        {
            if selection.head != point {
                selection.head = point;
                cx.notify();
            }
        }
    }

    pub(super) fn finish_text_selection(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.selecting_text = false;
    }
}

pub(super) fn selected_fragments(
    line: &TerminalLine,
    row: usize,
    selection: Option<&TerminalSelection>,
) -> Vec<SelectedFragment> {
    let mut fragments = Vec::<SelectedFragment>::new();
    let mut column = 0;
    for span in &line.spans {
        for character in span.text.chars() {
            let selected = selection
                .is_some_and(|selection| selection.contains(TerminalPoint { row, column }));
            if let Some(fragment) = fragments
                .last_mut()
                .filter(|fragment| fragment.style == span.style && fragment.selected == selected)
            {
                fragment.text.push(character);
            } else {
                fragments.push(SelectedFragment {
                    text: character.to_string(),
                    style: span.style.clone(),
                    selected,
                });
            }
            column += terminal_character_width(character);
        }
    }
    fragments
}

pub(super) fn line_width(line: &TerminalLine) -> usize {
    line.spans
        .iter()
        .flat_map(|span| span.text.chars())
        .map(terminal_character_width)
        .sum()
}

fn selected_text(frame: &TerminalFrame, selection: &TerminalSelection) -> String {
    let (start, end) = selection.range();
    let Some(lines) = frame
        .lines
        .get(start.row..=end.row.min(frame.lines.len().saturating_sub(1)))
    else {
        return String::new();
    };
    lines
        .iter()
        .enumerate()
        .map(|(line_offset, line)| {
            let row = start.row + line_offset;
            let mut column = 0;
            let mut selected = String::new();
            for character in line.spans.iter().flat_map(|span| span.text.chars()) {
                if selection.contains(TerminalPoint { row, column }) {
                    selected.push(character);
                }
                column += terminal_character_width(character);
            }
            selected
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn terminal_character_width(character: char) -> usize {
    UnicodeWidthChar::width(character).unwrap_or(0).max(1)
}
