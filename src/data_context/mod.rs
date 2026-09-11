mod core;
mod event;
mod gui;
mod infrastructure;
mod mapping;
mod mcp;
mod query;
mod validation;

pub type DataContextResult<T> = Result<T, String>;
pub(crate) use crate::application::model::{
    ProfileSummary, SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo, SftpTransferSummary,
    SftpWatchSummary, TerminalReadPage, TerminalSummary,
};
pub use core::DataContext;
#[allow(unused_imports)]
pub use event::DataContextEvent;
pub use gui::GuiContext;
pub(crate) use infrastructure::InfrastructureContext;
pub(crate) use mcp::McpContext;
pub(crate) use query::{ProfileQuery, QueryService};
