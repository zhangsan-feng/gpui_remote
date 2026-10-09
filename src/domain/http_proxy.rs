use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct HttpProxySettings {
    pub(crate) enabled: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) password: String,
}

impl Default for HttpProxySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_owned(),
            port: 8080,
            username: String::new(),
            password: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HttpProxyConnection {
    pub(crate) id: u64,
    pub(crate) client: SocketAddr,
    pub(crate) proxy: SocketAddr,
    pub(crate) destination: SocketAddr,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HttpProxyStatus {
    pub(crate) running: bool,
    pub(crate) address: Option<String>,
    pub(crate) active_connections: usize,
    pub(crate) connections: Vec<HttpProxyConnection>,
    pub(crate) error: Option<String>,
}
