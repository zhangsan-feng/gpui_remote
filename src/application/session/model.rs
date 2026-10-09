use serde::Serialize;

use crate::domain::session::ConnectionProtocol;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProfileSummary {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) host: String,
    pub(crate) protocol: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SessionSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) connection_protocol: ConnectionProtocol,
    pub(crate) ssh_tunnel_configured: bool,
    pub(crate) ssh_tunnel_active: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct SshTunnelWorkspaceSummary {
    pub(crate) workspace_id: String,
    pub(crate) profile_id: String,
    pub(crate) ip: String,
    pub(crate) title: String,
    pub(crate) remote_port: u16,
}
