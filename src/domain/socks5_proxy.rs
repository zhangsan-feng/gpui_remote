use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Socks5ProxySettings {
    pub(crate) enabled: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) password: String,
}

impl Default for Socks5ProxySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_owned(),
            port: 1080,
            username: String::new(),
            password: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Socks5ProxyConnection {
    pub(crate) id: u64,
    pub(crate) client: SocketAddr,
    pub(crate) proxy: SocketAddr,
    pub(crate) destination: SocketAddr,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Socks5ProxyStatus {
    pub(crate) running: bool,
    pub(crate) address: Option<String>,
    pub(crate) active_connections: usize,
    pub(crate) connections: Vec<Socks5ProxyConnection>,
    pub(crate) error: Option<String>,
}
