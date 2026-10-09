mod core;
mod external;
mod model;

pub(crate) use model::{McpSettings, McpTerminalReadPage, TerminalSummary};

#[derive(Clone, Copy, Default)]
pub(crate) struct McpApplication;

impl McpApplication {
    pub(crate) fn new() -> Self {
        Self
    }
}
