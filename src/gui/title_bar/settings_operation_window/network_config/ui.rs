use super::NetworkConfigEditor;
use crate::component::theme::CustomerUiTheme;
use gpui_kit::component::{
    ActiveTheme, Disableable, IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

impl NetworkConfigEditor {
    pub(super) fn render_view(&self, cx: &Context<Self>) -> impl IntoElement {
        let configs = crate::data_context::DATA_CONTEXT.network_configs();
        let entries: Vec<_> = if self.proxy {
            configs
                .proxies
                .iter()
                .map(|p| {
                    (
                        p.id.clone(),
                        p.name.clone(),
                        format!("{}:{}", p.config.host, p.config.port),
                    )
                })
                .collect()
        } else {
            configs
                .tunnels
                .iter()
                .map(|p| {
                    (
                        p.id.clone(),
                        p.name.clone(),
                        format!(
                            "{} → {}:{}",
                            p.config.remote_port, p.config.forward_address, p.config.forward_port
                        ),
                    )
                })
                .collect()
        };
        v_flex()
            .size_full()
            .min_h_0()
            .gap_4()
            .child(
                h_flex()
                    .flex_shrink_0()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(if self.proxy {
                                "代理配置"
                            } else {
                                "SSH 隧道配置"
                            }),
                    )
                    .child(
                        Button::new("new-network-config")
                            .outline()
                            .small()
                            .icon(IconName::Plus)
                            .label("新增配置")
                            .disabled(self.busy)
                            .on_click(
                                cx.listener(|this, _, window, cx| this.edit(None, window, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.proxy {
                        "统一管理 SOCKS 代理，在会话中选择使用。"
                    } else {
                        "统一管理反向转发参数，远端主机使用会话的连接地址。"
                    }),
            )
            .when(self.show_editor, |view| {
                view.child(
                    v_flex()
                        .flex_shrink_0()
                        .gap_3()
                        .p_4()
                        .rounded_xl()
                        .border_1()
                        .border_color(CustomerUiTheme::border_color(cx))
                        .bg(CustomerUiTheme::panel_background(cx))
                        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(
                            if self.editing.is_some() {
                                "编辑配置"
                            } else {
                                "新增配置"
                            },
                        ))
                        .child(self.field("配置名称", &self.name))
                        .child(self.field(
                            if self.proxy {
                                "代理主机"
                            } else {
                                "转发地址"
                            },
                            &self.address,
                        ))
                        .child(self.field(
                            if self.proxy {
                                "代理端口"
                            } else {
                                "远端端口"
                            },
                            &self.port,
                        ))
                        .when(self.proxy, |view| {
                            view.child(self.field("用户名", &self.username))
                                .child(self.field("密码", &self.password))
                        })
                        .when(!self.proxy, |view| {
                            view.child(self.field("转发端口", &self.forward_port))
                        })
                        .child(
                            h_flex()
                                .gap_2()
                                .justify_end()
                                .child(
                                    Button::new("cancel-network-config")
                                        .outline()
                                        .label("取消")
                                        .disabled(self.busy)
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.cancel_edit(cx)),
                                        ),
                                )
                                .child(
                                    Button::new("save-network-config")
                                        .primary()
                                        .label(if self.busy {
                                            "保存中…"
                                        } else {
                                            "保存配置"
                                        })
                                        .disabled(self.busy)
                                        .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                                ),
                        ),
                )
            })
            .when_some(self.message.clone(), |view, message| {
                view.child(
                    div()
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(if self.failed {
                            cx.theme().danger
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(message),
                )
            })
            .child(
                v_flex()
                    .id(if self.proxy {
                        "proxy-config-list"
                    } else {
                        "tunnel-config-list"
                    })
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .gap_4()
                    .overflow_y_scrollbar()
                    .when(entries.is_empty(), |view| {
                        view.child(
                            div()
                                .p_3()
                                .rounded_lg()
                                .bg(CustomerUiTheme::panel_background(cx))
                                .text_sm()
                                .child("暂无配置，点击“新增配置”开始添加。"),
                        )
                    })
                    .children(entries.into_iter().map(|(id, name, detail)| {
                        let selected = self.show_editor && self.editing.as_ref() == Some(&id);
                        let delete_id = id.clone();
                        h_flex()
                            .id(SharedString::from(format!("network-{id}")))
                            .flex_shrink_0()
                            .p_3()
                            .gap_3()
                            .rounded_lg()
                            .border_1()
                            .border_color(if selected {
                                cx.theme().primary
                            } else {
                                CustomerUiTheme::border_color(cx)
                            })
                            .bg(CustomerUiTheme::panel_background(cx))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(div().text_sm().truncate().child(name))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .truncate()
                                            .child(detail),
                                    ),
                            )
                            .child(
                                Button::new(SharedString::from(format!("edit-{id}")))
                                    .ghost()
                                    .small()
                                    .label("编辑")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.edit(Some(id.clone()), window, cx)
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("delete-{delete_id}")))
                                    .ghost()
                                    .small()
                                    .label("删除")
                                    .disabled(self.busy)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.delete(delete_id.clone(), cx);
                                    })),
                            )
                    })),
            )
    }

    fn field(&self, label: &'static str, input: &Entity<InputState>) -> Div {
        h_flex()
            .gap_3()
            .child(div().w(px(72.)).text_sm().child(label))
            .child(div().flex_1().child(Input::new(input).disabled(self.busy)))
    }
}
