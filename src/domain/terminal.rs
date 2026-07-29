use std::sync::Arc;
use tokio::sync::oneshot;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalRgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalStyle {
    pub foreground: TerminalRgb,
    pub background: TerminalRgb,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalSpan {
    pub text: String,
    pub style: TerminalStyle,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TerminalLine {
    pub number: Option<u64>,
    pub timestamp: Option<Arc<str>>,
    pub spans: Vec<TerminalSpan>,
    pub wrapped: bool,
}

#[derive(Clone, Debug, Default)]
pub struct TerminalFrame {
    pub lines: Arc<Vec<Arc<TerminalLine>>>,
    pub application_cursor: bool,
    pub history_size: usize,
    pub display_offset: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalStatus {
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

#[derive(Clone, Debug)]
pub struct TerminalData {
    pub frame: Arc<TerminalFrame>,
    pub status: TerminalStatus,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TerminalHistoryPage {
    pub text: String,
    pub total_lines: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

pub enum TerminalSessionCommand {
    Input(Vec<u8>),
    Resize {
        columns: u32,
        rows: u32,
    },
    Scroll {
        lines: i32,
    },
    ScrollTo {
        offset: usize,
    },
    Read {
        offset: usize,
        limit: usize,
        reply: oneshot::Sender<TerminalHistoryPage>,
    },
    Disconnect,
}
