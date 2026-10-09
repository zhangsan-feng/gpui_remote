use super::{ServicesOperationWindow, port_forward_core::PortForwardAction};
use crate::component::theme;
use gpui_kit::component::Disableable;
use gpui_kit::component::{
    ActiveTheme, Sizable,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

impl ServicesOperationWindow {
    pub(super) fn port_forward_section(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        let statuses = crate::data_context::DATA_CONTEXT.port_forward_statuses();
        v_flex()
            .gap_2()
            .child(self.section_heading(
                "端口转发",
                "将本机 TCP 端口转发到目标主机。启用的规则会在应用启动时自动运行。",
                cx,
            ))
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .child(format!("{} 条规则", self.port_forward_rules.len())),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                Button::new("refresh-port-forwards")
                                    .outline()
                                    .small()
                                    .label("刷新")
                                    .disabled(self.port_forward_busy)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh_port_forward_rules(cx)
                                    })),
                            )
                            .child(
                                Button::new("add-port-forward")
                                    .primary()
                                    .small()
                                    .label("新增规则")
                                    .disabled(self.port_forward_busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.edit_port_forward_rule(None, window, cx)
                                    })),
                            ),
                    ),
            )
            .when(self.port_forward_rules.is_empty(), |this| {
                this.child(
                    div()
                        .p_2()
                        .rounded_lg()
                        .border_1()
                        .border_color(theme::CustomerUiTheme::border_color(cx))
                        .text_sm()
                        .text_color(colors.muted_foreground)
                        .child(if self.port_forward_busy {
                            "正在加载规则…"
                        } else {
                            "暂无转发规则，点击“新增规则”配置本机和目标端点。"
                        }),
                )
            })
            .children(self.port_forward_rules.iter().map(|rule| {
                let status = statuses.get(&rule.id);
                let running = status.is_some_and(|status| status.running);
                let state = if running {
                    "运行中"
                } else if rule.enabled {
                    "启动失败 / 未运行"
                } else {
                    "已停止"
                };
                let connections = status.map_or(0, |status| status.active_connections);
                let active_routes =
                    status.map_or_else(Vec::new, |status| status.active_connection_routes.clone());
                let toggle_id = rule.id.clone();
                let retry_id = rule.id.clone();
                let delete_id = rule.id.clone();
                let confirm_id = rule.id.clone();
                let edit_rule = rule.clone();
                let enabled = rule.enabled;
                let confirming = self.port_forward_delete_confirmation.as_ref() == Some(&rule.id);
                v_flex()
                    .gap_3()
                    .p_2()
                    .rounded_xl()
                    .border_1()
                    .border_color(theme::CustomerUiTheme::border_color(cx))
                    .bg(theme::CustomerUiTheme::panel_background(cx))
                    .child(
                        h_flex().items_center().justify_between().gap_3().child(
                            v_flex().gap_1().min_w_0().flex_1().child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!(
                                        "{} → {}",
                                        endpoint(&rule.listen_host, rule.listen_port),
                                        endpoint(&rule.target_host, rule.target_port)
                                    )),
                            ),
                        ),
                    )
                    // .when_some(
                    //     status.and_then(|status| status.listen_address.clone()),
                    //     |this, address| {
                    //         this.child(
                    //             div()
                    //                 .text_xs()
                    //                 .text_color(colors.muted_foreground)
                    //                 .child(format!("实际监听：{address}")),
                    //         )
                    //     },
                    // )
                    .when_some(
                        status.and_then(|status| status.error.clone()),
                        |this, error| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(colors.danger_foreground)
                                    .child(error),
                            )
                        },
                    )
                    .when(enabled, |this| {
                        this.child(
                            v_flex()
                                .gap_1()
                                .p_2()
                                .rounded_lg()
                                .border_1()
                                .border_color(theme::CustomerUiTheme::border_color(cx))
                                .bg(theme::CustomerUiTheme::panel_background(cx))
                                // .child(
                                //     div()
                                //         .text_xs()
                                //         .font_weight(FontWeight::SEMIBOLD)
                                //         .child("活动连接（最新 5 条）"),
                                // )
                                .when(active_routes.is_empty(), |this| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(colors.muted_foreground)
                                            .child("暂无活动连接"),
                                    )
                                })
                                .children(active_routes.iter().take(5).map(|route| {
                                    div().text_xs().text_color(colors.muted_foreground).child(
                                        format!(
                                            "{} → {}",
                                            route.client_address, route.target_address
                                        ),
                                    )
                                })),
                        )
                    })
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.muted_foreground)
                                    .child(format!("{state} · {connections} 个活动连接")),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .when(enabled && !running, |this| {
                                        this.child(
                                            Button::new(format!("retry-forward-{}", rule.id))
                                                .outline()
                                                .small()
                                                .label("重试启动")
                                                .disabled(self.port_forward_busy)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.apply_port_forward_action(
                                                        PortForwardAction::Retry(retry_id.clone()),
                                                        cx,
                                                    )
                                                })),
                                        )
                                    })
                                    .child(
                                        Button::new(format!("toggle-forward-{}", rule.id))
                                            .small()
                                            .when(enabled, |button| button.primary())
                                            .when(!enabled, |button| button.outline())
                                            .label(if enabled { "停用" } else { "启用" })
                                            .disabled(self.port_forward_busy)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.apply_port_forward_action(
                                                    PortForwardAction::Enable(
                                                        toggle_id.clone(),
                                                        !enabled,
                                                    ),
                                                    cx,
                                                )
                                            })),
                                    )
                                    .child(h_flex().gap_2().when(!enabled, |this| {
                                        this.child(
                                            Button::new(format!("edit-forward-{}", rule.id))
                                                .outline()
                                                .small()
                                                .label("编辑")
                                                .disabled(self.port_forward_busy)
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.edit_port_forward_rule(
                                                            Some(edit_rule.clone()),
                                                            window,
                                                            cx,
                                                        )
                                                    },
                                                )),
                                        )
                                        .child(
                                            Button::new(format!("delete-forward-{}", rule.id))
                                                .ghost()
                                                .small()
                                                .label("删除")
                                                .disabled(self.port_forward_busy)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.port_forward_delete_confirmation =
                                                        Some(delete_id.clone());
                                                    cx.notify();
                                                })),
                                        )
                                    })),
                            ),
                    )
                    // .when(enabled, |this| {
                    //     this.child(
                    //         div()
                    //             .text_xs()
                    //             .text_color(colors.muted_foreground)
                    //             .child("停用后可编辑或删除此规则。"),
                    //     )
                    // })
                    .when(confirming, |this| {
                        this.child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(colors.danger_foreground)
                                        .child("确认删除此规则？删除后无法恢复。"),
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            Button::new(format!(
                                                "cancel-delete-forward-{}",
                                                rule.id
                                            ))
                                            .outline()
                                            .small()
                                            .label("取消")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.port_forward_delete_confirmation = None;
                                                cx.notify();
                                            })),
                                        )
                                        .child(
                                            Button::new(format!(
                                                "confirm-delete-forward-{}",
                                                rule.id
                                            ))
                                            .danger()
                                            .small()
                                            .label("确认删除")
                                            .disabled(self.port_forward_busy || enabled)
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.apply_port_forward_action(
                                                    PortForwardAction::Delete(confirm_id.clone()),
                                                    cx,
                                                )
                                            })),
                                        ),
                                ),
                        )
                    })
            }))
            .when_some(self.port_forward_error.clone(), |this, error| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(colors.danger_foreground)
                        .child(error),
                )
            })
    }
}

fn endpoint(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
