mod core;
mod event;
mod gui;
mod infrastructure;
mod mcp;
mod model;
mod query;

pub type DataContextResult<T> = Result<T, String>;
pub use core::DataContext;
#[allow(unused_imports)]
pub use event::DataContextEvent;
pub use gui::GuiContext;
pub use infrastructure::InfrastructureContext;
pub use mcp::McpContext;
pub use model::{
    ProfileSummary, SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo, SftpTransferSummary,
    SftpWatchSummary, TerminalReadPage, TerminalSummary,
};
pub use query::{ProfileQuery, QueryService};
