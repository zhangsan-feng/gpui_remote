mod bridge;
mod core;
mod event;
mod external;
pub(crate) mod mapping;
pub mod model;
mod session;
mod sftp;
mod ssh;
pub(crate) mod validation;

pub(crate) use bridge::start_mcp_bridge;
pub use core::ApplicationContext;
pub use event::ApplicationEvent;
pub use session::SessionApplication;
pub(crate) use sftp::LocalWatchSummary;
pub(crate) use sftp::SftpApplication;
pub(crate) use sftp::{
    LocalSnapshot, RemoteDeleteItem, SftpSnapshot, SftpStatus, TransferRecord, TransferRequest,
};
pub(crate) use ssh::SshApplication;

pub type ApplicationResult<T> = Result<T, String>;
