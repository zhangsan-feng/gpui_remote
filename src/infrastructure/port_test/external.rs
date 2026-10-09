use crate::{application::ports::PortTestFuture, infrastructure::InfrastructureContext};

impl InfrastructureContext {
    pub(crate) fn probe_tcp_port(&self, host: String, port: u16) -> PortTestFuture<()> {
        Box::pin(async move { probe_tcp_port(host, port).await })
    }
}

pub(in crate::infrastructure) async fn probe_tcp_port(
    host: String,
    port: u16,
) -> Result<(), String> {
    super::core::probe_tcp_port(host, port).await
}
