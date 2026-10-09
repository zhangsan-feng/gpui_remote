use gpui_kit::component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputContentType},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::component::theme;

use super::ServicesOperationWindow;

const MAX_VISIBLE_CONNECTIONS: usize = 5;

impl ServicesOperationWindow {
    pub(super) fn socks5_proxy_section(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        let status = crate::data_context::DATA_CONTEXT.socks5_proxy();
        let status_label = if status.running {
            format!(
                "运行中 · {} · {} 个连接",
                status.address.as_deref().unwrap_or_default(),
                status.active_connections
            )
        } else {
            "未运行".to_owned()
        };
        v_flex()
            .gap_5()
            .child(self.section_heading(
                "SOCKS5 代理",
                "提供本机 SOCKS5 CONNECT 代理，默认使用无认证模式。",
                cx,
            ))
            .child(
                v_flex()
                    .p_2()
                    .gap_2()
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
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("服务状态"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(colors.muted_foreground)
                                            .child(status_label),
                                    ),
                            )
                            .child(
                                Button::new("toggle-socks5-proxy")
                                    .when(self.socks5_enabled, |button| button.primary())
                                    .when(!self.socks5_enabled, |button| button.outline())
                                    .label(if self.socks5_enabled {
                                        "已启用"
                                    } else {
                                        "已关闭"
                                    })
                                    .on_click(cx.listener(Self::toggle_socks5_proxy)),
                            ),
                    )
                    .child(Self::mcp_field("监听地址", &self.socks5_host, colors))
                    .child(Self::mcp_field("端口", &self.socks5_port, colors))
                    .child(Self::mcp_field("用户名", &self.socks5_username, colors))
                    .child(
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
                                    .child("密码"),
                            )
                            .child(
                                div().flex_1().min_w_0().child(
                                    Input::new(&self.socks5_password)
                                        .small()
                                        .content_type(InputContentType::Password),
                                ),
                            ),
                    )
                    .child(
                        h_flex().justify_end().pt_2().child(
                            Button::new("apply-socks5-proxy")
                                .primary()
                                .label("保存并应用")
                                .on_click(cx.listener(Self::apply_socks5_proxy)),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("活跃连接"),
                                    )
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .when(
                                                status.connections.len() > MAX_VISIBLE_CONNECTIONS,
                                                |this| {
                                                    this.child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(colors.muted_foreground)
                                                            .child(format!(
                                                                "显示最新 {} 条",
                                                                MAX_VISIBLE_CONNECTIONS
                                                            )),
                                                    )
                                                },
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(colors.muted_foreground)
                                                    .child(format!(
                                                        "{} 条",
                                                        status.connections.len()
                                                    )),
                                            ),
                                    ),
                            )
                            .when(status.connections.is_empty(), |this| {
                                this.child(
                                    div()
                                        .w_full()
                                        .p_3()
                                        .rounded_lg()
                                        .border_1()
                                        .border_color(theme::CustomerUiTheme::border_color(cx))
                                        .text_xs()
                                        .text_color(colors.muted_foreground)
                                        .child("当前没有活跃连接"),
                                )
                            })
                            .children(
                                status
                                    .connections
                                    .iter()
                                    .rev()
                                    .take(MAX_VISIBLE_CONNECTIONS)
                                    .map(|connection| {
                                        div()
                                            .w_full()
                                            .p_3()
                                            .rounded_lg()
                                            .border_1()
                                            .border_color(theme::CustomerUiTheme::border_color(cx))
                                            .text_xs()
                                            .child(format!(
                                                "客户端 {} → 我的代理 {} → 目的 {}",
                                                connection.client,
                                                connection.proxy,
                                                connection.destination
                                            ))
                                    }),
                            ),
                    )
                    .when_some(status.error, |this, error| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.danger_foreground)
                                .child(error),
                        )
                    })
                    .when_some(self.socks5_error.clone(), |this, error| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.danger_foreground)
                                .child(error),
                        )
                    }),
            )
    }
}
