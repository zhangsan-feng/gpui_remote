mod database;
mod http_proxy;
mod lifecycle;
pub(crate) mod mcp;
pub(crate) mod port_forward;
pub(crate) mod port_test;
pub(crate) mod ports;
pub(crate) mod session;
pub(crate) mod sftp;
pub(crate) mod sftp_server;
mod socks5_proxy;
mod ssh;
mod ssh_server;
mod theme;

pub(crate) type ApplicationResult<T> = Result<T, String>;

use std::sync::{Arc, LazyLock, atomic::AtomicBool};

pub(crate) static APPLICATION: LazyLock<Application> = LazyLock::new(Application::new);

/// Concrete application services grouped by module. Clones share the state owned by each service.
#[derive(Clone)]
pub(crate) struct Application {
    pub(crate) sessions: session::SessionApplication,
    pub(crate) database: database::DatabaseApplication,
    pub(crate) ssh: ssh::SshApplication,
    pub(crate) sftp: sftp::SftpApplication,
    pub(crate) ssh_server: ssh_server::SshServerApplication,
    pub(crate) sftp_server: Arc<sftp_server::SftpServerApplication>,
    pub(crate) http_proxy: http_proxy::HttpProxyApplication,
    pub(crate) socks5_proxy: socks5_proxy::Socks5ProxyApplication,
    pub(crate) port_forward: port_forward::PortForwardApplication,
    pub(crate) port_test: port_test::PortTestApplication,
    pub(crate) theme: theme::ThemeApplication,
    pub(crate) mcp: mcp::McpApplication,
    shutting_down: Arc<AtomicBool>,
    operations: Arc<tokio::sync::RwLock<()>>,
}

impl Application {
    fn new() -> Self {
        let sessions = session::SessionApplication::new();
        Self {
            sessions: sessions.clone(),
            database: database::DatabaseApplication::new(),
            ssh: ssh::SshApplication::new(sessions.clone()),
            sftp: sftp::SftpApplication::new(sessions),
            ssh_server: ssh_server::SshServerApplication::new(),
            sftp_server: Arc::new(sftp_server::SftpServerApplication::new()),
            http_proxy: http_proxy::HttpProxyApplication::new(),
            socks5_proxy: socks5_proxy::Socks5ProxyApplication::new(),
            port_forward: port_forward::PortForwardApplication::new(),
            port_test: port_test::PortTestApplication::new(),
            theme: theme::ThemeApplication::new(),
            mcp: mcp::McpApplication::new(),
            shutting_down: Arc::new(AtomicBool::new(false)),
            operations: Arc::new(tokio::sync::RwLock::new(())),
        }
    }
}
