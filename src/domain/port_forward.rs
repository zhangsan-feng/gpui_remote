use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PortForwardRule {
    pub(crate) id: String,
    pub(crate) listen_host: String,
    pub(crate) listen_port: u16,
    pub(crate) target_host: String,
    pub(crate) target_port: u16,
    #[serde(default)]
    pub(crate) enabled: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PortForwardConnection {
    pub(crate) client_address: String,
    pub(crate) target_address: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PortForwardStatus {
    pub(crate) rule_id: String,
    pub(crate) running: bool,
    pub(crate) listen_address: Option<String>,
    pub(crate) active_connections: usize,
    #[serde(default)]
    pub(crate) active_connection_routes: Vec<PortForwardConnection>,
    pub(crate) error: Option<String>,
}
