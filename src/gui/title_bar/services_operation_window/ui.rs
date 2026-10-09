use crate::component::theme;
use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable, ThemeColor,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::{ServicesOperationWindow, ServicesSection};

impl ServicesOperationWindow {
    pub(super) fn render_view(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = theme::CustomerUiTheme::colors(cx);
        let content = match self.active_section {
            ServicesSection::Mcp => self.mcp_section(cx).into_any_element(),
            ServicesSection::PortForward => self.port_forward_section(cx).into_any_element(),
            ServicesSection::HttpProxy => self.http_proxy_section(cx).into_any_element(),
            ServicesSection::PortTest => self.port_test_section(cx).into_any_element(),
            ServicesSection::SshServer => self.ssh_server_section(cx).into_any_element(),
            ServicesSection::Socks5Proxy => self.socks5_proxy_section(cx).into_any_element(),
        };
        v_flex()
            .size_full()
            .bg(colors.background)
            .text_color(colors.text_color)
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.window_title_bar(cx))
                    .child(
                        h_flex()
                            .flex_1()
                            .min_h_0()
                            .items_stretch()
                            .child(self.sidebar(cx))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .bg(Hsla::transparent_black())
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .overflow_y_scrollbar()
                                            .p_6()
                                            .gap_5()
                                            .child(content),
                                    ),
                            ),
                    ),
            )
    }

    fn window_title_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = theme::CustomerUiTheme::colors(cx);
        let accent = cx.theme().accent;
        let danger = cx.theme().danger;
        h_flex()
            .id("services-window-titlebar")
            .w_full()
            .h(px(42.))
            .flex_shrink_0()
            .items_center()
            .border_b_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::title_background(cx))
            .child(
                div()
                    .id("services-window-titlebar-drag")
                    .h_full()
                    .flex_1()
                    .flex()
                    .items_center()
                    .px_4()
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.text_color)
                            .child("服务"),
                    ),
            )
            .child(
                h_flex()
                    .h_full()
                    .border_l_1()
                    .border_color(theme::CustomerUiTheme::border_color(cx))
                    .child(
                        div()
                            .id("services-window-minimize")
                            .size(px(34.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(colors.text_color)
                            .hover(|style| style.bg(accent))
                            .window_control_area(WindowControlArea::Min)
                            .when(cfg!(target_os = "linux"), |this| {
                                this.on_click(
                                    cx.listener(|_, _, window, _| window.minimize_window()),
                                )
                            })
                            .child("−"),
                    )
                    .child(
                        div()
                            .id("services-window-close")
                            .size(px(34.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(colors.text_color)
                            .hover(|style| style.bg(danger))
                            .on_click(cx.listener(|_, _, window, _| window.remove_window()))
                            .child("×"),
                    ),
            )
    }

    fn sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(184.))
            .flex_shrink_0()
            .p_3()
            .gap_2()
            .border_r_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::sidebar_background(cx))
            .child(
                div()
                    .px_3()
                    .pt_2()
                    .pb_3()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("服务"),
            )
            .child(self.sidebar_item(ServicesSection::Mcp, "MCP 服务", cx))
            .child(self.sidebar_item(ServicesSection::PortTest, "端口测试", cx))
            .child(self.sidebar_item(ServicesSection::SshServer, "SSH 服务", cx))
            .child(self.sidebar_item(ServicesSection::PortForward, "端口转发", cx))
            .child(self.sidebar_item(ServicesSection::HttpProxy, "HTTP 代理", cx))
            .child(self.sidebar_item(ServicesSection::Socks5Proxy, "SOCKS5 代理", cx))
    }

    fn sidebar_item(
        &self,
        section: ServicesSection,
        label: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let ui_colors = theme::CustomerUiTheme::colors(cx);
        let selected = self.active_section == section;
        h_flex()
            .id(match section {
                ServicesSection::Mcp => "services-section-mcp",
                ServicesSection::PortForward => "services-section-port-forward",
                ServicesSection::HttpProxy => "services-section-http-proxy",
                ServicesSection::PortTest => "services-section-port-test",
                ServicesSection::SshServer => "services-section-ssh-server",
                ServicesSection::Socks5Proxy => "services-section-socks5-proxy",
            })
            .h(px(38.))
            .px_3()
            .gap_2()
            .rounded_lg()
            .when(selected, |this| {
                this.bg(ui_colors.select_background)
                    .text_color(ui_colors.text_color)
            })
            .when(!selected, |this| {
                this.text_color(cx.theme().sidebar_foreground)
                    .hover(|style| style.bg(ui_colors.hover_background))
            })
            .cursor_pointer()
            .child(Icon::new(IconName::Settings2).small())
            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_section = section;
                cx.notify();
            }))
    }
    fn mcp_section(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_5()
            .child(self.section_heading(
                "MCP 服务",
                "MCP 服务监听本机所有网络接口；局域网客户端需使用本机 IP，建议启用 Token 验证。",
                cx,
            ))
            .child(self.mcp_panel(cx))
    }

    fn mcp_panel(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        v_flex()
            .p_2()
            .gap_3()
            .rounded_xl()
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("服务状态"),
                            )
                            .child(div().text_xs().text_color(colors.muted_foreground).child(
                                if self.mcp_enabled {
                                    "应用启动时自动运行，开关会立即启停服务"
                                } else {
                                    "服务已关闭，开关会立即启停服务"
                                },
                            )),
                    )
                    .child(
                        Button::new("toggle-mcp-enabled")
                            .when(self.mcp_enabled, |this| this.primary())
                            .when(!self.mcp_enabled, |this| this.outline())
                            .label(if self.mcp_busy {
                                "应用中…"
                            } else if self.mcp_enabled {
                                "已启动"
                            } else {
                                "已停止"
                            })
                            .disabled(self.mcp_busy)
                            .on_click(cx.listener(Self::toggle_mcp_enabled)),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Token 验证"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.muted_foreground)
                                    .child("开启后，访问 MCP 必须携带 Authorization Bearer Token"),
                            ),
                    )
                    .child(
                        Button::new("toggle-mcp-token-enabled")
                            .when(self.mcp_token_enabled, |this| this.primary())
                            .when(!self.mcp_token_enabled, |this| this.outline())
                            .label(if self.mcp_token_enabled {
                                "已启用"
                            } else {
                                "未启用"
                            })
                            .disabled(self.mcp_busy)
                            .on_click(cx.listener(Self::toggle_mcp_token_enabled)),
                    ),
            )
            .child(Self::fixed_listener_field("监听范围", colors))
            .child(Self::mcp_field("Port", &self.mcp_port, colors))
            .child(self.mcp_token_field(cx))
            .when(!self.mcp_token_enabled, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(colors.danger_foreground)
                        .child("Token 验证未启用，局域网内可访问本机的设备能够调用 MCP 服务。"),
                )
            })
            .child(
                h_flex().justify_end().pt_2().child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("copy-mcp-config")
                                .outline()
                                .label("复制 MCP 配置")
                                .disabled(self.mcp_busy)
                                .on_click(cx.listener(Self::copy_mcp_config)),
                        )
                        .child(
                            Button::new("apply-mcp-settings")
                                .primary()
                                .label("保存并重启服务")
                                .disabled(self.mcp_busy)
                                .on_click(cx.listener(Self::apply_mcp_settings)),
                        ),
                ),
            )
            .when_some(self.mcp_error.clone(), |this, error| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(colors.danger_foreground)
                        .child(error),
                )
            })
    }

    pub(super) fn mcp_field(
        label: &'static str,
        input: &Entity<InputState>,
        colors: ThemeColor,
    ) -> Div {
        h_flex()
            .w_full()
            .h(px(34.))
            .gap_2()
            .items_center()
            .child(
                div()
                    .w(px(80.))
                    .flex_shrink_0()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.muted_foreground)
                    .child(label),
            )
            .child(div().flex_1().child(Input::new(input).small()))
    }

    pub(super) fn fixed_listener_field(label: &'static str, colors: ThemeColor) -> Div {
        h_flex()
            .w_full()
            .h(px(34.))
            .gap_2()
            .items_center()
            .child(
                div()
                    .w(px(80.))
                    .flex_shrink_0()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.muted_foreground)
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .child("所有网络接口（0.0.0.0）"),
            )
    }

    fn mcp_token_field(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        h_flex()
            .w_full()
            .h(px(34.))
            .gap_2()
            .items_center()
            .child(
                div()
                    .w(px(80.))
                    .flex_shrink_0()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.muted_foreground)
                    .child("Token"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(30.))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .border_1()
                    .border_color(theme::CustomerUiTheme::border_color(cx))
                    .bg(theme::CustomerUiTheme::panel_background(cx))
                    .text_xs()
                    .text_color(colors.foreground)
                    .truncate()
                    .child(self.mcp_token.clone()),
            )
    }

    pub(super) fn section_heading(
        &self,
        title: &'static str,
        description: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(description),
            )
    }
}
