use crate::{
    application::{Application, ApplicationResult},
    domain::network_config::NetworkConfigs,
};

impl Application {
    pub(crate) async fn initialize_network_configs(&self) -> ApplicationResult<()> {
        super::core::initialize().await
    }

    pub(crate) async fn save_network_config(
        &self,
        proxy: bool,
        id: String,
        name: String,
        config: NetworkConfigs,
        delete: bool,
    ) -> ApplicationResult<()> {
        super::core::save(proxy, id, name, config, delete).await
    }
}
