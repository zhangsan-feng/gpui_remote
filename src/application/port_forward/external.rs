use crate::{application::ApplicationResult, domain::port_forward::PortForwardRule};

use super::{PortForwardApplication, PortForwardRuleDraft};

impl PortForwardApplication {
    pub(crate) async fn list(&self) -> ApplicationResult<Vec<PortForwardRule>> {
        self.list_internal().await
    }

    pub(crate) async fn create(
        &self,
        draft: PortForwardRuleDraft,
    ) -> ApplicationResult<PortForwardRule> {
        self.create_internal(draft).await
    }

    pub(crate) async fn update(
        &self,
        id: String,
        draft: PortForwardRuleDraft,
    ) -> ApplicationResult<PortForwardRule> {
        self.update_internal(id, draft).await
    }

    pub(crate) async fn delete(&self, id: String) -> ApplicationResult<()> {
        self.delete_internal(id).await
    }

    pub(crate) async fn set_enabled(&self, id: String, enabled: bool) -> ApplicationResult<()> {
        self.set_enabled_internal(id, enabled).await
    }

    pub(crate) async fn retry(&self, id: String) -> ApplicationResult<()> {
        self.retry_internal(id).await
    }

    pub(crate) async fn initialize(&self) -> ApplicationResult<()> {
        self.initialize_internal().await
    }

    pub(crate) async fn shutdown(&self) {
        self.shutdown_internal().await;
    }
}
