use super::DataContext;
use crate::domain::network_config::NetworkConfigs;
use crate::event_bus::{EVENT_BUS, refresh_gui::GuiComponent};

impl DataContext {
    pub(crate) fn network_configs(&self) -> NetworkConfigs {
        self.network_configs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub(crate) fn set_network_configs(&self, configs: NetworkConfigs) {
        *self
            .network_configs
            .write()
            .unwrap_or_else(|e| e.into_inner()) = configs;
        EVENT_BUS.refresh_gui.notify(&[GuiComponent::NetworkConfig]);
    }
}
