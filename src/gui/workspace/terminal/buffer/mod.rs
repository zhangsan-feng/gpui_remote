mod color;
mod core;

pub(super) use color::default_background;

use std::{collections::VecDeque, sync::Arc};

use alacritty_terminal::{
    event::{Event, EventListener},
    grid::Dimensions,
    term::Term,
    vte::ansi,
};
use tokio::sync::mpsc;

use crate::domain::terminal::TerminalSessionCommand;

const DEFAULT_COLUMNS: usize = 120;
const DEFAULT_ROWS: usize = 36;
const MAX_SCROLLBACK_LINES: usize = 5_000;
const MAX_TRACKED_LINE_METADATA: usize = MAX_SCROLLBACK_LINES + DEFAULT_ROWS * 2;

#[derive(Clone, Copy)]
struct TerminalSize {
    columns: usize,
    rows: usize,
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

#[derive(Clone, Default)]
struct TerminalPtyProxy {
    commands: Option<mpsc::UnboundedSender<TerminalSessionCommand>>,
}

impl EventListener for TerminalPtyProxy {
    fn send_event(&self, event: Event) {
        if let (Some(commands), Event::PtyWrite(text)) = (&self.commands, event) {
            let _ = commands.send(TerminalSessionCommand::Input(text.into_bytes()));
        }
    }
}

pub(super) struct TerminalBuffer {
    parser: ansi::Processor,
    terminal: Term<TerminalPtyProxy>,
    current_line_number: u64,
    first_tracked_line_number: u64,
    line_timestamps: VecDeque<Arc<str>>,
}
