use std::collections::HashMap;

use tokio::sync::watch;

use crate::{
    application::ports::PortForwardFuture,
    domain::port_forward::{PortForwardRule, PortForwardStatus},
    infrastructure::InfrastructureContext,
};

impl InfrastructureContext {
    pub(crate) fn load_port_forward_rules(&self) -> PortForwardFuture<Vec<PortForwardRule>> {
        let controller = self.inner.port_forward.clone();
        Box::pin(async move { controller.load().await })
    }

    pub(crate) fn save_port_forward_rules(
        &self,
        rules: Vec<PortForwardRule>,
    ) -> PortForwardFuture<()> {
        let controller = self.inner.port_forward.clone();
        Box::pin(async move { controller.save(rules).await })
    }

    pub(crate) fn start_port_forward(
        &self,
        rule: PortForwardRule,
    ) -> PortForwardFuture<PortForwardStatus> {
        let controller = self.inner.port_forward.clone();
        Box::pin(async move { controller.start(rule).await })
    }

    pub(crate) fn stop_port_forward(&self, rule_id: String) -> PortForwardFuture<()> {
        let controller = self.inner.port_forward.clone();
        Box::pin(async move { controller.stop(rule_id).await })
    }

    pub(crate) fn shutdown_port_forwards(&self) -> PortForwardFuture<()> {
        let controller = self.inner.port_forward.clone();
        Box::pin(async move { controller.shutdown().await })
    }

    pub(crate) fn subscribe_port_forward_status(
        &self,
    ) -> watch::Receiver<HashMap<String, PortForwardStatus>> {
        self.inner.port_forward.status.subscribe()
    }
}
