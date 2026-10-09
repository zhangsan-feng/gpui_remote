use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct McpSettings {
    pub(crate) enabled: bool,
    pub(crate) token_enabled: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) token: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TerminalSummary {
    pub(crate) workspace_id: String,
    pub(crate) profile_id: String,
    pub(crate) ip: String,
    pub(crate) title: String,
    pub(crate) host: String,
    pub(crate) protocol: String,
    pub(crate) status: String,
    pub(crate) selected: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct McpTerminalReadPage {
    pub(crate) workspace_id: String,
    pub(crate) text: String,
    pub(crate) total_lines: usize,
    pub(crate) offset: usize,
    pub(crate) limit: usize,
    pub(crate) has_more: bool,
    pub(crate) mcp_snapshot_version: u64,
    pub(crate) changed: bool,
}
