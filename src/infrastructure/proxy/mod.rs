mod core;
mod external;

use std::{future::Future, net::SocketAddr, pin::Pin};

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::{sync::oneshot, task::JoinHandle};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxySettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

pub trait ProxyStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> ProxyStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

pub type ConnectionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn ProxyStream>>> + Send + 'a>>;

/// Adapts one network route into an async byte stream for a remote endpoint.
///
/// Database drivers and SSH/SFTP transports can depend on this interface
/// instead of knowing how direct or proxied connections are established.
pub trait ConnectionAdapter: Send + Sync {
    fn connect<'a>(&'a self, target: (&'a str, u16)) -> ConnectionFuture<'a>;
}

/// A local loopback listener that forwards every accepted connection through
/// an SSH tunnel to the configured remote target.
pub struct LocalPortForward {
    local_addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl LocalPortForward {
    pub(crate) fn new(
        local_addr: SocketAddr,
        shutdown: oneshot::Sender<()>,
        task: JoinHandle<()>,
    ) -> Self {
        Self {
            local_addr,
            shutdown: Some(shutdown),
            task: Some(task),
        }
    }

    /// Returns the local endpoint that database drivers can connect to.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stops accepting new connections and closes active forwarded streams.
    pub async fn shutdown(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for LocalPortForward {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DirectAdapter;

#[derive(Clone, Debug)]
pub struct Socks5Adapter {
    settings: ProxySettings,
}

impl Socks5Adapter {
    pub fn new(settings: ProxySettings) -> Self {
        Self { settings }
    }
}

#[derive(Clone, Debug)]
pub struct SshTunnelSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub private_key_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SshTunnelAdapter {
    settings: SshTunnelSettings,
}

impl SshTunnelAdapter {
    pub fn new(settings: SshTunnelSettings) -> Self {
        Self { settings }
    }
}

pub use external::connect;
