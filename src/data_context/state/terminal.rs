use std::sync::Arc;

use crate::domain::terminal::TerminalData;

use super::super::terminal::TerminalBuffer;

#[derive(Clone, Debug)]
pub(crate) struct TerminalReadSnapshot {
    pub(crate) data: Arc<TerminalData>,
    pub(crate) mcp_snapshot_version: u64,
    pub(crate) gui_snapshot_version: u64,
}

pub(in crate::data_context) struct TerminalWorkspaceData {
    pub(in crate::data_context) snapshot: TerminalReadSnapshot,
    pub(in crate::data_context) buffer: Option<TerminalBuffer>,
}
