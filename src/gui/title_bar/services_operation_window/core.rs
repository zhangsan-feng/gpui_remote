use gpui_kit::*;

use crate::application::mcp::McpSettings;

use super::ServicesOperationWindow;

impl ServicesOperationWindow {
    pub(super) fn toggle_mcp_enabled(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mcp_enabled = !self.mcp_enabled;
        cx.notify();
    }

    pub(super) fn toggle_mcp_token_enabled(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mcp_token_enabled = !self.mcp_token_enabled;
        cx.notify();
    }

    pub(super) fn copy_mcp_config(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let host = self.mcp_host.read(cx).value().trim().to_owned();
        let port = self.mcp_port.read(cx).value().trim().parse::<u16>();

        let result = if host.is_empty() {
            Err("MCP Host 不能为空".to_owned())
        } else if port.as_ref().is_err() || port == Ok(0) {
            Err("MCP Port 必须是 1-65535 的数字".to_owned())
        } else if self.mcp_token_enabled && self.mcp_token.is_empty() {
            Err("MCP Token 不能为空".to_owned())
        } else {
            let port = port.expect("MCP 端口已校验");
            let server = if self.mcp_token_enabled {
                serde_json::json!({
                    "url": format!("http://{host}:{port}/mcp"),
                    "headers": {
                        "Authorization": format!("Bearer {}", self.mcp_token),
                    },
                    "description": "本地 MCP 服务，用于通过 SSH/SFTP 操作远程主机",
                })
            } else {
                serde_json::json!({
                    "url": format!("http://{host}:{port}/mcp"),
                    "description": "本地 MCP 服务，用于通过 SSH/SFTP 操作远程主机",
                })
            };
            let config = serde_json::json!({
                "mcpServers": {
                    "gpui-remote": server,
                },
            });
            serde_json::to_string(&config).map_err(|error| error.to_string())
        };

        match result {
            Ok(config) => {
                cx.write_to_clipboard(ClipboardItem::new_string(config));
                self.mcp_error = None;
            }
            Err(error) => {
                self.mcp_error = Some(error);
            }
        }
        cx.notify();
    }

    pub(super) fn apply_mcp_settings(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let host = self.mcp_host.read(cx).value().trim().to_owned();
        let port = self.mcp_port.read(cx).value().trim().parse::<u16>();
        let port = match port {
            Ok(port) if port > 0 => port,
            _ => {
                self.mcp_error = Some("MCP Port 必须是 1-65535 的数字".to_owned());
                cx.notify();
                return;
            }
        };
        let token = self.mcp_token.clone();
        if host.is_empty() {
            self.mcp_error = Some("MCP Host 不能为空".to_owned());
            cx.notify();
            return;
        }
        if self.mcp_token_enabled && token.is_empty() {
            self.mcp_error = Some("MCP Token 不能为空".to_owned());
            cx.notify();
            return;
        }

        let application = crate::application::APPLICATION.clone();
        let settings = McpSettings {
            enabled: self.mcp_enabled,
            token_enabled: self.mcp_token_enabled,
            host,
            port,
            token,
        };
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.mcp.update_settings(settings).await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(settings) => {
                        this.mcp_token = settings.token;
                        this.mcp_error = None;
                    }
                    Err(error) => {
                        this.mcp_error = Some(error);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}
