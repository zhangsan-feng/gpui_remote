mod database;
mod local_filesystem;
mod mcp_settings;
mod port_forward;
mod session_repository;
mod sftp;
mod socks5_proxy;
mod ssh;
mod ssh_server;
mod theme_settings;

pub(crate) use database::{DatabaseConnectFuture, DatabaseConnection};
pub(crate) use local_filesystem::{LocalEntry, LocalFsFuture, LocalWatchSource};
pub(crate) use mcp_settings::McpSettingsFuture;
pub(crate) use port_forward::PortForwardFuture;
pub(crate) use session_repository::SessionRepositoryFuture;
pub(crate) use sftp::{
    RemoteDeleteItem, SftpCancellationCallback, SftpConnectFuture, SftpConnection, SftpEntry,
    SftpFuture, SftpProgressCallback, SftpTransferFuture, SftpTransferOutcome,
};
pub(crate) use socks5_proxy::Socks5ProxyFuture;
pub(crate) use ssh::{SshChannelEvent, SshOpenFuture, SshResultFuture, SshShell, SshWaitFuture};
pub(crate) use ssh_server::SshServerFuture;
pub(crate) use theme_settings::ThemeSettingsFuture;

use std::{future::Future, pin::Pin};

pub(crate) type PortTestFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send>>;
