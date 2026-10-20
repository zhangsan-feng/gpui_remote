use super::NetworkConfigEditor;
use crate::domain::{
    network_config::{NamedConfig, NetworkConfigs},
    session::{ProxyConfig, SshReverseTunnelConfig},
};
use gpui_kit::*;

impl NetworkConfigEditor {
    pub(super) fn edit(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let configs = crate::data_context::DATA_CONTEXT.network_configs();
        let mut name = String::new();
        let mut address = if self.proxy {
            String::new()
        } else {
            "127.0.0.1".into()
        };
        let mut port = if self.proxy {
            "1080".into()
        } else {
            String::new()
        };
        let mut username = String::new();
        let mut password = String::new();
        let mut forward_port = String::new();
        if let Some(id) = &id {
            if self.proxy {
                if let Some(p) = configs.proxies.iter().find(|p| &p.id == id) {
                    name = p.name.clone();
                    address = p.config.host.clone();
                    port = p.config.port.to_string();
                    username = p.config.username.clone();
                    password = p.config.password.clone();
                }
            } else if let Some(p) = configs.tunnels.iter().find(|p| &p.id == id) {
                name = p.name.clone();
                address = p.config.forward_address.clone();
                port = p.config.remote_port.to_string();
                forward_port = p.config.forward_port.to_string();
            }
        }
        self.editing = id;
        self.show_editor = true;
        for (input, value) in [
            (&self.name, name),
            (&self.address, address),
            (&self.port, port),
            (&self.username, username),
            (&self.password, password),
            (&self.forward_port, forward_port),
        ] {
            input.update(cx, |input, cx| input.set_value(value, window, cx));
        }
        self.message = None;
        cx.notify();
    }

    fn draft(&self, id: String, name: String, cx: &App) -> Result<NetworkConfigs, String> {
        let port = |input: &Entity<gpui_kit::component::input::InputState>| -> Result<u16, String> {
            input
                .read(cx)
                .value()
                .trim()
                .parse::<u16>()
                .ok()
                .filter(|p| *p > 0)
                .ok_or_else(|| "端口范围为 1–65535".into())
        };
        let mut configs = NetworkConfigs::default();
        if self.proxy {
            configs.proxies.push(NamedConfig {
                id,
                name,
                config: ProxyConfig {
                    host: self.address.read(cx).value().trim().into(),
                    port: port(&self.port)?,
                    username: self.username.read(cx).value().trim().into(),
                    password: self.password.read(cx).value().to_string(),
                },
            });
        } else {
            configs.tunnels.push(NamedConfig {
                id,
                name,
                config: SshReverseTunnelConfig {
                    remote_port: port(&self.port)?,
                    forward_address: self.address.read(cx).value().trim().into(),
                    forward_port: port(&self.forward_port)?,
                },
            });
        }
        Ok(configs)
    }

    pub(super) fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.show_editor = false;
        self.editing = None;
        self.message = None;
        cx.notify();
    }

    pub(super) fn delete(&mut self, id: String, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.persist(id, String::new(), NetworkConfigs::default(), true, cx);
    }

    pub(super) fn save(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let id = self
            .editing
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let name = self.name.read(cx).value().trim().to_owned();
        let configs = self.draft(id.clone(), name.clone(), cx);
        let configs = match configs {
            Ok(configs) => configs,
            Err(error) => {
                self.message = Some(error);
                self.failed = true;
                cx.notify();
                return;
            }
        };
        self.persist(id, name, configs, false, cx);
    }

    fn persist(
        &mut self,
        id: String,
        name: String,
        configs: NetworkConfigs,
        delete: bool,
        cx: &mut Context<Self>,
    ) {
        let proxy = self.proxy;
        self.busy = true;
        self.message = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let saved_id = id.clone();
            let result = crate::global_state::run_application(async move {
                crate::application::APPLICATION
                    .save_network_config(proxy, id, name, configs, delete)
                    .await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                this.failed = result.is_err();
                match result {
                    Ok(()) => {
                        if !delete || this.editing.as_ref() == Some(&saved_id) {
                            this.editing = None;
                            this.show_editor = false;
                        }
                        this.message = Some(
                            if delete {
                                "配置已删除；已保存会话仍保留原参数"
                            } else {
                                "配置已保存，可在会话中选择"
                            }
                            .into(),
                        );
                    }
                    Err(error) => {
                        log::error!("network_config_save_failed: {error}");
                        this.message = Some(error);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}
