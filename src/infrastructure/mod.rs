pub(crate) mod agent_mcp;
mod connection;
mod database;
mod http_proxy;
mod lifecycle;
mod local_fs;
pub(crate) mod logging;
mod port_forward;
mod port_test;
pub mod proxy;
mod sftp;
mod sftp_server;
mod socks5_proxy;
mod ssh;
mod ssh_server;
pub mod storage;
mod theme;

use std::sync::{Arc, LazyLock};
use storage::Storage;

pub(crate) static INFRASTRUCTURE: LazyLock<InfrastructureContext> =
    LazyLock::new(InfrastructureContext::new);

struct InfrastructureContextInner {
    storage: tokio::sync::OnceCell<Storage>,
    mcp: agent_mcp::McpRuntime,
    ssh_server: ssh_server::SshServerController,
    ssh_reverse_tunnel: ssh::reverse_tunnel::SshReverseTunnelController,
    sftp_server_filesystem: Arc<sftp_server::LocalSftpFilesystem>,
    socks5_proxy: socks5_proxy::Socks5ProxyController,
    http_proxy: http_proxy::HttpProxyController,
    port_forward: Arc<port_forward::PortForwardController>,
}

#[derive(Clone)]
pub struct InfrastructureContext {
    inner: Arc<InfrastructureContextInner>,
}

impl InfrastructureContext {
    fn new() -> Self {
        Self {
            inner: Arc::new(InfrastructureContextInner {
                storage: tokio::sync::OnceCell::new(),
                mcp: agent_mcp::McpRuntime::new(),
                ssh_server: ssh_server::SshServerController::new(),
                ssh_reverse_tunnel: ssh::reverse_tunnel::SshReverseTunnelController::new(),
                sftp_server_filesystem: sftp_server::LocalSftpFilesystem::new(),
                socks5_proxy: socks5_proxy::Socks5ProxyController::new(),
                http_proxy: http_proxy::HttpProxyController::new(),
                port_forward: port_forward::PortForwardController::new(),
            }),
        }
    }
}
