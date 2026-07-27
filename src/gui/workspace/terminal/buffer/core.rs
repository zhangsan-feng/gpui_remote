use std::sync::Arc;

use alacritty_terminal::{
    grid::{Dimensions, Scroll},
    index::{Column, Line},
    term::{Config, Osc52, TermMode, cell::Flags},
    vte::ansi,
};

use crate::domain::terminal::{
    TerminalFrame, TerminalLine, TerminalSessionCommand, TerminalSpan, TerminalStyle,
};

use super::{
    DEFAULT_COLUMNS, DEFAULT_ROWS, MAX_SCROLLBACK_LINES, MAX_TRACKED_LINE_METADATA, TerminalBuffer,
    TerminalPtyProxy, TerminalSize,
    color::{default_background, resolve_color},
};

impl TerminalBuffer {
    pub(in crate::gui::workspace::terminal) fn new(
        commands: tokio::sync::mpsc::UnboundedSender<TerminalSessionCommand>,
    ) -> Self {
        Self::with_event_proxy(TerminalPtyProxy {
            commands: Some(commands),
        })
    }

    fn with_event_proxy(event_proxy: TerminalPtyProxy) -> Self {
        let config = Config {
            scrolling_history: MAX_SCROLLBACK_LINES,
            osc52: Osc52::Disabled,
            ..Default::default()
        };
        Self {
            parser: ansi::Processor::new(),
            terminal: alacritty_terminal::term::Term::new(
                config,
                &TerminalSize {
                    columns: DEFAULT_COLUMNS,
                    rows: DEFAULT_ROWS,
                },
                event_proxy,
            ),
            current_line_number: 1,
            first_tracked_line_number: 1,
            line_timestamps: std::collections::VecDeque::new(),
        }
    }

    pub(in crate::gui::workspace::terminal) fn process(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let was_alternate_screen = self.terminal.mode().contains(TermMode::ALT_SCREEN);
        self.parser.advance(&mut self.terminal, bytes);
        let is_alternate_screen = self.terminal.mode().contains(TermMode::ALT_SCREEN);
        if was_alternate_screen || is_alternate_screen {
            return;
        }

        let timestamp: Arc<str> = chrono::Local::now().format("%H:%M:%S").to_string().into();
        if self.line_timestamps.is_empty() {
            self.line_timestamps.push_back(timestamp.clone());
        }
        for _ in bytes.iter().filter(|byte| **byte == b'\n') {
            self.current_line_number = self.current_line_number.saturating_add(1);
            self.line_timestamps.push_back(timestamp.clone());
            if self.line_timestamps.len() > MAX_TRACKED_LINE_METADATA {
                self.line_timestamps.pop_front();
                self.first_tracked_line_number = self.first_tracked_line_number.saturating_add(1);
            }
        }
    }

    pub(in crate::gui::workspace::terminal) fn resize(&mut self, columns: u32, rows: u32) {
        self.terminal.resize(TerminalSize {
            columns: columns.max(2) as usize,
            rows: rows.max(1) as usize,
        });
    }

    pub(in crate::gui::workspace::terminal) fn scroll(&mut self, lines: i32) {
        if lines != 0 {
            self.terminal.scroll_display(Scroll::Delta(lines));
        }
    }

    pub(in crate::gui::workspace::terminal) fn scroll_to(&mut self, offset: usize) {
        let current_offset = self.terminal.grid().display_offset();
        let delta = offset as i64 - current_offset as i64;
        if delta != 0 {
            self.terminal.scroll_display(Scroll::Delta(
                delta.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            ));
        }
    }

    pub(in crate::gui::workspace::terminal) fn frame_reusing(
        &self,
        previous: Option<&TerminalFrame>,
    ) -> TerminalFrame {
        let grid = self.terminal.grid();
        let lines = (0..self.terminal.screen_lines())
            .filter_map(|index| {
                let line = self.line(index)?;
                previous
                    .and_then(|frame| frame.lines.get(index))
                    .filter(|previous_line| previous_line.as_ref() == &line)
                    .cloned()
                    .or_else(|| Some(Arc::new(line)))
            })
            .collect();
        TerminalFrame {
            lines: Arc::new(lines),
            application_cursor: self.terminal.mode().contains(TermMode::APP_CURSOR),
            history_size: grid.total_lines().saturating_sub(grid.screen_lines()),
            display_offset: grid.display_offset(),
        }
    }

    fn line(&self, index: usize) -> Option<TerminalLine> {
        let grid = self.terminal.grid();
        if index >= grid.screen_lines() {
            return None;
        }
        let colors = *self.terminal.renderable_content().colors;
        let cursor = grid.cursor.point;
        let show_cursor =
            grid.display_offset() == 0 && self.terminal.mode().contains(TermMode::SHOW_CURSOR);
        let row = index as i32 - grid.display_offset() as i32;
        let logical_line_number =
            self.current_line_number as i64 + i64::from(row) - i64::from(cursor.line.0);
        let timestamp = if self.terminal.mode().contains(TermMode::ALT_SCREEN) {
            None
        } else {
            u64::try_from(logical_line_number)
                .ok()
                .filter(|number| *number > 0)
                .and_then(|number| self.line_timestamp(number).cloned())
        };
        let number = timestamp.as_ref().map(|_| logical_line_number as u64);
        let mut spans = Vec::<TerminalSpan>::new();
        for column in 0..grid.columns() {
            let cell = &grid[Line(row)][Column(column)];
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let mut foreground = resolve_color(cell.fg, &colors, true, cell.flags);
            let mut background = resolve_color(cell.bg, &colors, false, cell.flags);
            let is_cursor = show_cursor && cursor.line.0 == row && cursor.column.0 == column;
            if cell.flags.contains(Flags::INVERSE) || is_cursor {
                std::mem::swap(&mut foreground, &mut background);
            }
            if cell.flags.contains(Flags::HIDDEN) {
                foreground = background;
            }
            let style = TerminalStyle {
                foreground,
                background,
                bold: cell.flags.contains(Flags::BOLD),
                italic: cell.flags.contains(Flags::ITALIC),
                underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
            };
            let mut character = String::from(cell.c);
            if let Some(zerowidth) = cell.zerowidth() {
                character.extend(zerowidth);
            }
            if let Some(span) = spans.last_mut().filter(|span| span.style == style) {
                span.text.push_str(&character);
            } else {
                spans.push(TerminalSpan {
                    text: character,
                    style,
                });
            }
        }
        trim_default_trailing_spaces(&mut spans);
        Some(TerminalLine {
            number,
            timestamp,
            spans,
        })
    }

    fn line_timestamp(&self, line_number: u64) -> Option<&Arc<str>> {
        let index = line_number.checked_sub(self.first_tracked_line_number)? as usize;
        self.line_timestamps.get(index)
    }
}

fn trim_default_trailing_spaces(spans: &mut Vec<TerminalSpan>) {
    let default_background = default_background();
    while let Some(last) = spans.last_mut() {
        if last.style.background != default_background {
            break;
        }
        let trimmed_length = last.text.trim_end_matches(' ').len();
        last.text.truncate(trimmed_length);
        if last.text.is_empty() {
            spans.pop();
        } else {
            break;
        }
    }
}
