use gpui::*;
use std::sync::Arc;
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
    frame: Arc<TerminalFrame>,
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
        let text = selected_text(&selection.frame, selection);
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
        let frame = self
            .model(&workspace_id)
            .map(|model| model.read().frame.clone())
            .unwrap_or_default();
        self.selection = Some(TerminalSelection {
            workspace_id,
            anchor: point,
            head: point,
            frame,
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
    let selection = selection.filter(|selection| {
        let (start, end) = selection.range();
        start.row <= row && row <= end.row
    });
    if selection.is_none() {
        return line
            .spans
            .iter()
            .map(|span| SelectedFragment {
                text: span.text.clone(),
                style: span.style.clone(),
                selected: false,
            })
            .collect();
    }

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

pub(super) fn nearest_character_column(line: &TerminalLine, target: usize) -> usize {
    let mut column = 0;
    for character in line.spans.iter().flat_map(|span| span.text.chars()) {
        let width = terminal_character_width(character);
        if width > 0 && target < column + width {
            return column;
        }
        column += width;
    }
    column.saturating_sub(1)
}

fn selected_text(frame: &TerminalFrame, selection: &TerminalSelection) -> String {
    let (start, end) = selection.range();
    let Some(lines) = frame
        .lines
        .get(start.row..=end.row.min(frame.lines.len().saturating_sub(1)))
    else {
        return String::new();
    };
    let mut text = String::new();
    for (line_offset, line) in lines.iter().enumerate() {
        let row = start.row + line_offset;
        let mut column = 0;
        for character in line.spans.iter().flat_map(|span| span.text.chars()) {
            if selection.contains(TerminalPoint { row, column }) {
                text.push(character);
            }
            column += terminal_character_width(character);
        }
        if line_offset + 1 < lines.len() && !line.wrapped {
            text.push('\n');
        }
    }
    text
}

fn terminal_character_width(character: char) -> usize {
    UnicodeWidthChar::width(character).unwrap_or(0)
}
