use crate::{application::ports::PortTestFuture, infrastructure::InfrastructureContext};

use super::PortTestProbeResult;

impl InfrastructureContext {
    pub(crate) fn probe_tcp_port(
        &self,
        host: String,
        port: u16,
    ) -> PortTestFuture<PortTestProbeResult> {
        Box::pin(async move { probe_tcp_port(host, port).await })
    }
}

pub(in crate::infrastructure) async fn probe_tcp_port(
    host: String,
    port: u16,
) -> Result<PortTestProbeResult, String> {
    Ok(super::core::probe_tcp_port(host, port).await)
}
