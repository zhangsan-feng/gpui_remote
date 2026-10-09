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

impl ServicesOperationWindow {
    pub(super) fn ssh_server_section(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        let status = crate::data_context::DATA_CONTEXT.ssh_server();
        let status_label = if status.running {
            format!("运行中 · {}", status.address.unwrap_or_default())
        } else {
            "未运行".to_owned()
        };
        v_flex()
            .gap_5()
            .child(self.section_heading(
                "SSH 服务",
                "允许密码验证的客户端连接本机交互式 Shell 和 SFTP 文件服务。",
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
                                Button::new("toggle-ssh-server")
                                    .when(self.ssh_enabled, |button| button.primary())
                                    .when(!self.ssh_enabled, |button| button.outline())
                                    .label(if self.ssh_enabled {
                                        "已启用"
                                    } else {
                                        "已关闭"
                                    })
                                    .on_click(cx.listener(Self::toggle_ssh_enabled)),
                            ),
                    )
                    // .child(
                    //     div().text_xs().text_color(colors.muted_foreground).child(
                    //         "默认关闭。启用后监听所填地址；客户端使用下方用户名和密码登录，SFTP 根目录默认是 Desktop（不存在时为用户目录）。",
                    //     ),
                    // )
                    .child(Self::mcp_field("监听地址", &self.ssh_host, colors))
                    .child(Self::mcp_field("端口", &self.ssh_port, colors))
                    .child(Self::mcp_field("用户名", &self.ssh_username, colors))
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
                                    .child("登录密码"),
                            )
                            .child(
                                div().flex_1().min_w_0().child(
                                    Input::new(&self.ssh_password)
                                        .small()
                                        .content_type(InputContentType::Password),
                                ),
                            )
                            .child(
                                Button::new("generate-ssh-password")
                                    .outline()
                                    .label("随机生成")
                                    .on_click(cx.listener(Self::generate_ssh_password)),
                            )
                            .child(
                                Button::new("copy-ssh-password")
                                    .outline()
                                    .label("复制")
                                    .on_click(cx.listener(Self::copy_ssh_password)),
                            ),
                    )
                    .child(
                        h_flex().justify_end().pt_2().child(
                            Button::new("apply-ssh-settings")
                                .primary()
                                .label("保存并应用")
                                .on_click(cx.listener(Self::apply_ssh_settings)),
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
                    .when_some(self.ssh_error.clone(), |this, error| {
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
