use gpui_kit::component::{
    ActiveTheme, Disableable,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::component::theme;

use super::ServicesOperationWindow;

impl ServicesOperationWindow {
    pub(super) fn port_test_section(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        v_flex()
            .gap_5()
            .child(self.section_heading(
                "端口测试",
                "直接测试本机到目标地址的 TCP 连接，不经过 HTTP 代理或端口转发。",
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
                    .child(Self::mcp_field("主机", &self.port_test_host, colors))
                    .child(Self::mcp_field("端口", &self.port_test_port, colors))
                    .child(
                        h_flex().justify_end().pt_2().child(
                            Button::new("run-port-test")
                                .primary()
                                .label(if self.port_test_busy {
                                    "正在测试…"
                                } else {
                                    "测试端口"
                                })
                                .disabled(self.port_test_busy)
                                .on_click(cx.listener(Self::run_port_test)),
                        ),
                    )
                    .when_some(self.port_test_error.clone(), |this, error| {
                        this.child(div().text_xs().text_color(colors.foreground).child(error))
                    })
                    .when_some(self.port_test_result.clone(), |this, result| {
                        let endpoint = if result.host.contains(':') {
                            format!("[{}]:{}", result.host, result.port)
                        } else {
                            format!("{}:{}", result.host, result.port)
                        };
                        let (label, status_background, status_foreground) = if result.reachable {
                            ("端口可达", colors.success, colors.success_foreground)
                        } else {
                            ("端口不可达", colors.danger, colors.danger_foreground)
                        };
                        this.child(
                            v_flex()
                                .gap_1()
                                .pt_1()
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .px_2()
                                                .py_1()
                                                .rounded_md()
                                                .bg(status_background)
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(status_foreground)
                                                .child(label),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(colors.foreground)
                                                .child(endpoint),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(colors.muted_foreground)
                                        .child(format!("耗时：{} ms", result.elapsed_ms)),
                                )
                                .when_some(result.error, |this, error| {
                                    this.child(
                                        div().text_xs().text_color(colors.foreground).child(error),
                                    )
                                }),
                        )
                    }),
            )
    }
}
