use gpui_kit::component::{
    ActiveTheme, Disableable, Sizable,
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
    pub(super) fn http_proxy_section(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        let status = crate::data_context::DATA_CONTEXT.http_proxy();
        let status_error = status.error.clone();
        let local_error = self
            .http_proxy_error
            .clone()
            .filter(|error| status_error.as_ref() != Some(error));
        let status_label = if status.running {
            "运行中"
        } else {
            "已停止"
        };

        v_flex()
            .gap_5()
            .child(self.section_heading(
                "HTTP 代理",
                "提供 HTTP 和 HTTPS CONNECT 代理供局域网客户端使用；非回环监听需要设置账号和密码。",
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
                                Button::new("toggle-http-proxy")
                                    .when(self.http_proxy_enabled, |button| button.primary())
                                    .when(!self.http_proxy_enabled, |button| button.outline())
                                    .label(if self.http_proxy_enabled {
                                        "已启用"
                                    } else {
                                        "已关闭"
                                    })
                                    .disabled(self.http_proxy_loading || self.http_proxy_busy)
                                    .on_click(cx.listener(Self::toggle_http_proxy)),
                            ),
                    )
                    .child(Self::fixed_listener_field("监听范围", colors))
                    .child(Self::http_proxy_field(
                        "端口",
                        &self.http_proxy_port,
                        self.http_proxy_loading,
                        colors,
                    ))
                    .child(Self::http_proxy_field(
                        "用户名",
                        &self.http_proxy_username,
                        self.http_proxy_loading,
                        colors,
                    ))
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
                                    Input::new(&self.http_proxy_password)
                                        .small()
                                        .disabled(self.http_proxy_loading)
                                        .content_type(InputContentType::Password),
                                ),
                            ),
                    )
                    .child(
                        h_flex().justify_end().pt_2().child(
                            Button::new("apply-http-proxy")
                                .primary()
                                .label("保存并重启")
                                .disabled(self.http_proxy_loading || self.http_proxy_busy)
                                .on_click(cx.listener(Self::apply_http_proxy)),
                        ),
                    )
                    // .when_some(status.address, |this, address| {
                    //     this.child(
                    //         div()
                    //             .text_xs()
                    //             .text_color(colors.muted_foreground)
                    //             .child(format!("实际监听：{address}")),
                    //     )
                    // })
                    // .child(
                    //     div()
                    //         .text_xs()
                    //         .text_color(colors.muted_foreground)
                    //         .child(format!("客户端会话：{}", status.active_connections)),
                    // )
                    .child(
                        v_flex()
                            .gap_2()
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
                                        .p_2()
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
                                            .p_2()
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
                    .when_some(status_error, |this, error| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.danger_foreground)
                                .child(error),
                        )
                    })
                    .when_some(local_error, |this, error| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.danger_foreground)
                                .child(error),
                        )
                    }),
            )
    }

    fn http_proxy_field(
        label: &'static str,
        input: &Entity<gpui_kit::component::input::InputState>,
        loading: bool,
        colors: gpui_kit::component::ThemeColor,
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
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(input).small().disabled(loading)),
            )
    }
}
