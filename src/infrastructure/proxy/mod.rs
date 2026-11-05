mod core;
mod external;

use std::{future::Future, pin::Pin};

use anyhow::Result;

pub use super::connection::ConnectionStream as ProxyStream;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxySettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

pub type ConnectionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn ProxyStream>>> + Send + 'a>>;

/// Adapts one network route into an async byte stream for a remote endpoint.
///
/// Database drivers and SSH/SFTP transports can depend on this interface
/// instead of knowing how direct or proxied connections are established.
pub trait ConnectionAdapter: Send + Sync {
    fn connect<'a>(&'a self, target: (&'a str, u16)) -> ConnectionFuture<'a>;
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

pub use external::connect;
