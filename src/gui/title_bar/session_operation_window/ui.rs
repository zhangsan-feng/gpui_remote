use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    v_flex, ActiveTheme, Icon, IconName, Sizable, ThemeColor,
};

use crate::component::theme;

use super::{ConnectionProtocol, FormSection, SessionFormMode, SessionOperationWindow};

impl ConnectionProtocol {
    fn description(self) -> &'static str {
        match self {
            Self::Ssh => "安全终端",
            Self::Sftp => "安全文件传输",
            Self::Telnet => "传统终端",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Ssh => IconName::SquareTerminal,
            Self::Sftp => IconName::FolderOpen,
            Self::Telnet => IconName::Globe,
        }
    }
}

impl FormSection {
    const ALL: [Self; 2] = [Self::Connection, Self::Proxy];

    fn label(self) -> &'static str {
        match self {
            Self::Connection => "连接",
            Self::Proxy => "代理",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Connection => "主机与登录信息",
            Self::Proxy => "代理服务器设置",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Connection => IconName::SquareTerminal,
            Self::Proxy => IconName::Settings2,
        }
    }
}

impl SessionOperationWindow {
    fn protocol_option(&self, protocol: ConnectionProtocol, cx: &Context<Self>) -> AnyElement {
        let colors = cx.theme().colors;
        let styles = theme::styles(cx);
        let selected = self.protocol == protocol;
        let icon_color = if selected {
            colors.accent
        } else {
            colors.muted_foreground
        };

        div()
            .id(format!("protocol-{}", protocol.label()))
            .flex_1()
            .px_3()
            .py_2()
            .rounded_lg()
            .border_1()
            .border_color(if selected {
                colors.primary
            } else {
                theme::border_color(cx)
            })
            .bg(if selected {
                styles.selected
            } else {
                theme::panel_background(cx)
            })
            .cursor_pointer()
            .hover(|style| style.bg(styles.hover))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .size(px(30.))
                            .rounded_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(if selected {
                                colors.accent
                            } else {
                                styles.hover
                            })
                            .child(Icon::new(protocol.icon()).small().text_color(icon_color)),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.foreground)
                                    .child(protocol.label()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.muted_foreground)
                                    .child(protocol.description()),
                            ),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_protocol(protocol, cx);
            }))
            .into_any_element()
    }

    fn field(label: &'static str, input: &Entity<InputState>, colors: ThemeColor) -> Div {
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

    fn connection_panel(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        v_flex()
            .flex_1()
            .h_full()
            .p_3()
            .gap_2()
            .rounded_xl()
            .border_1()
            .border_color(theme::border_color(cx))
            .bg(theme::panel_background(cx))
            .child(Self::panel_heading(
                IconName::SquareTerminal,
                "连接信息",
                "配置远程主机与登录凭据",
                colors,
            ))
            .child(Self::field("标题", &self.name, colors))
            .child(Self::field("主机", &self.host, colors))
            .child(Self::field("端口", &self.port, colors))
            .child(Self::field("用户名", &self.username, colors))
            .child(Self::field("密码", &self.password, colors))
    }

    fn proxy_panel(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        v_flex()
            .flex_1()
            .h_full()
            .p_3()
            .gap_2()
            .rounded_xl()
            .border_1()
            .border_color(theme::border_color(cx))
            .bg(theme::panel_background(cx))
            .child(Self::panel_heading(
                IconName::Settings2,
                "代理",
                "可选的 SOCKS 或跳板代理配置",
                colors,
            ))
            .child(Self::field("代理主机", &self.proxy_host, colors))
            .child(Self::field("代理端口", &self.proxy_port, colors))
            .child(Self::field("代理用户名", &self.proxy_username, colors))
            .child(Self::field("代理密码", &self.proxy_password, colors))
    }

    fn panel_heading(
        icon: IconName,
        title: &'static str,
        description: &'static str,
        colors: ThemeColor,
    ) -> Div {
        h_flex()
            .gap_2()
            .items_center()
            .pb_1()
            .child(
                div()
                    .size(px(30.))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(colors.accent)
                    .child(Icon::new(icon).small().text_color(colors.accent_foreground)),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.foreground)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(description),
                    ),
            )
    }

    fn section_option(&self, section: FormSection, cx: &Context<Self>) -> AnyElement {
        let colors = cx.theme();
        let styles = theme::styles(cx);
        let selected = self.section == section;
        let label = section.label();

        div()
            .id(format!("session-section-{label}"))
            .w_full()
            .px_3()
            .py_2()
            .rounded_lg()
            .cursor_pointer()
            .bg(if selected {
                styles.selected
            } else {
                theme::panel_background(cx)
            })
            .border_1()
            .border_color(if selected {
                colors.primary
            } else {
                theme::border_color(cx)
            })
            .hover(|style| style.bg(styles.hover))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(section.icon()).small().text_color(if selected {
                        colors.accent
                    } else {
                        colors.muted_foreground
                    }))
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(if selected {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .text_color(colors.foreground)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.muted_foreground)
                                    .child(section.description()),
                            ),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_section(section, cx);
            }))
            .into_any_element()
    }

    pub(super) fn render_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let styles = theme::styles(cx);
        v_flex()
            .gap_2()
            .p_2()
            .size_full()
            .bg(styles.window_background)
            // .child(v_flex().p_2().gap_2().child(h_flex().gap_2().children(
            //     ConnectionProtocol::ALL.map(|protocol| self.protocol_option(protocol, cx)),
            // )))
            .child(
                h_flex()
                    .flex_1()
                    .p_2()
                    .gap_2()
                    .items_stretch()
                    .child(
                        v_flex()
                            .w(px(144.))
                            .flex_shrink_0()
                            .p_2()
                            .gap_2()
                            .rounded_xl()
                            .border_1()
                            .border_color(theme::border_color(cx))
                            .bg(theme::panel_background(cx))
                            .children(
                                FormSection::ALL.map(|section| self.section_option(section, cx)),
                            ),
                    )
                    .child(match self.section {
                        FormSection::Connection => self.connection_panel(cx),
                        FormSection::Proxy => self.proxy_panel(cx),
                    }),
            )
            .when_some(self.error.clone(), |this, error| {
                this.child(
                    h_flex()
                        .p_2()
                        .gap_2()
                        .rounded_md()
                        .border_1()
                        .border_color(colors.danger)
                        .bg(colors.danger.opacity(0.12))
                        .text_color(colors.danger)
                        .text_xs()
                        .child(IconName::TriangleAlert)
                        .child(error),
                )
            })
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_2()
                    .p_2()
                    .items_center()
                    .justify_end()
                    .border_t_1()
                    .border_color(theme::border_color(cx))
                    .bg(theme::panel_background(cx))
                    .child(
                        Button::new("cancel-session-operation")
                            .outline()
                            .label("取消")
                            .on_click(cx.listener(Self::cancel)),
                    )
                    .child(
                        Button::new("save-session-operation")
                            .primary()
                            .label(match &self.mode {
                                SessionFormMode::Create => "保存会话",
                                SessionFormMode::Edit { .. } => "保存修改",
                            })
                            .on_click(cx.listener(Self::submit)),
                    ),
            )
    }
}
