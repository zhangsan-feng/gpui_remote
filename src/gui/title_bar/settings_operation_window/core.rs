use gpui_kit::*;

use crate::infrastructure::{InfrastructureContext, agent_mcp::McpSettings};

use super::SettingsOperationWindow;

impl SettingsOperationWindow {
    pub(super) fn reset_font_color(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::component::theme::CustomerUiColor::clear_font_color(cx);
        self.sync_color_pickers(window, cx);
        cx.notify();
    }

    pub(super) fn reset_background_color(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::component::theme::CustomerUiColor::clear_background_color(cx);
        self.sync_color_pickers(window, cx);
        cx.notify();
    }

    pub(super) fn reset_hover_color(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::component::theme::CustomerUiColor::clear_hover_color(cx);
        self.sync_color_pickers(window, cx);
        cx.notify();
    }

    pub(super) fn reset_selected_color(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::component::theme::CustomerUiColor::clear_selected_color(cx);
        self.sync_color_pickers(window, cx);
        cx.notify();
    }

    pub(super) fn choose_wallpaper(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.wallpaper_error = None;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("选择背景图片".into()),
        });
        let this = cx.weak_entity();
        window
            .spawn(cx, async move |cx| {
                let paths = match receiver.await {
                    Ok(Ok(Some(paths))) => paths,
                    _ => return Ok::<(), anyhow::Error>(()),
                };
                let Some(path) = paths.into_iter().next() else {
                    return Ok(());
                };
                cx.update(|_, cx| {
                    let result = crate::component::theme::CustomerUiColor::set_wallpaper(&path, cx);
                    let _ = this.update(cx, |this, cx| {
                        this.wallpaper_error = result.err();
                        cx.notify();
                    });
                })?;
                Ok(())
            })
            .detach();
    }

    pub(super) fn clear_wallpaper(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.wallpaper_error = None;
        crate::component::theme::CustomerUiColor::clear_wallpaper(cx);
        cx.notify();
    }

    pub(super) fn toggle_mcp_enabled(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mcp_enabled = !self.mcp_enabled;
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
        } else if self.mcp_token.is_empty() {
            Err("MCP Token 不能为空".to_owned())
        } else {
            let port = port.expect("MCP 端口已校验");
            let config = serde_json::json!({
                "mcpServers": {
                    "gpui-remote": {
                        "url": format!("http://{host}:{port}/mcp"),
                        "headers": {
                            "Authorization": format!("Bearer {}", self.mcp_token),
                        },
                        "description": "本地 MCP 服务，用于通过 SSH/SFTP 操作远程主机",
                    },
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
        let token = self.mcp_token.clone();

        let result = if host.is_empty() {
            Err("MCP Host 不能为空".to_owned())
        } else if port.as_ref().is_err() || port == Ok(0) {
            Err("MCP Port 必须是 1-65535 的数字".to_owned())
        } else if token.is_empty() {
            Err("MCP Token 不能为空".to_owned())
        } else {
            cx.read_global::<InfrastructureContext, _>(|infrastructure, _| {
                infrastructure.apply_settings(McpSettings {
                    enabled: self.mcp_enabled,
                    host,
                    port: port.expect("MCP 端口已校验"),
                    token,
                })
            })
        };

        match result {
            Ok(settings) => {
                self.mcp_token = settings.token;
                self.mcp_error = None;
            }
            Err(error) => {
                self.mcp_error = Some(error);
            }
        }
        cx.notify();
    }
}
