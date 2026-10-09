use std::collections::HashMap;

use crate::domain::{
    http_proxy::HttpProxyStatus,
    port_forward::PortForwardStatus,
    session::{ConnectionProtocol, Protocol},
    socks5_proxy::Socks5ProxyStatus,
    ssh_server::SshServerStatus,
    terminal::TerminalStatus,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceSummary {
    pub(crate) workspace_id: String,
    pub(crate) profile_id: String,
    pub(crate) connection_protocol: ConnectionProtocol,
    pub(crate) title: String,
    pub(crate) host: String,
    pub(crate) protocol: Protocol,
    pub(crate) status: TerminalStatus,
    pub(crate) terminal_revision: u64,
    pub(crate) sftp_revision: u64,
    pub(crate) database_revision: u64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DataSnapshot {
    pub(crate) revision: u64,
    pub(crate) workspaces: Vec<WorkspaceSummary>,
    pub(crate) selected_workspace_id: Option<String>,
    pub(crate) ssh_server: SshServerStatus,
    pub(crate) socks5_proxy: Socks5ProxyStatus,
    pub(crate) http_proxy: HttpProxyStatus,
    pub(crate) port_forward_statuses: HashMap<String, PortForwardStatus>,
}

#[derive(Clone, Debug)]
pub(crate) enum DataChange {
    SessionOpened {
        workspace_id: String,
        profile: WorkspaceSummary,
    },
    SessionClosed {
        workspace_id: String,
    },
    SessionSelected {
        workspace_id: Option<String>,
    },
}
