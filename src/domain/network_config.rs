use super::session::{ProxyConfig, SshReverseTunnelConfig};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct NamedConfig<T> {
    pub id: String,
    pub name: String,
    pub config: T,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct NetworkConfigs {
    pub proxies: Vec<NamedConfig<ProxyConfig>>,
    pub tunnels: Vec<NamedConfig<SshReverseTunnelConfig>>,
}
