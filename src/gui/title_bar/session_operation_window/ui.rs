use gpui::*;
use gpui_component::{
    Icon, IconName, Sizable, h_flex,
    input::{Input, InputState},
    v_flex,
};

use crate::component::color::rgb_to_u32;

use super::{ConnectionProtocol, FormSection, SessionOperationWindow};

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
    pub(super) const ALL: [Self; 2] = [Self::Connection, Self::Proxy];

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
    pub(super) fn field(label: &'static str, input: &Entity<InputState>) -> Div {
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
                    .text_color(rgb_to_u32(71, 85, 105))
                    .child(label),
            )
            .child(div().flex_1().child(Input::new(input).small()))
    }

    pub(super) fn protocol_option(
        &self,
        protocol: ConnectionProtocol,
        cx: &Context<Self>,
    ) -> AnyElement {
        let selected = self.protocol == protocol;
        let icon_color = if selected {
            rgb_to_u32(109, 40, 217)
        } else {
            rgb_to_u32(100, 116, 139)
        };

        div()
            .id(format!("protocol-{}", protocol.label()))
            .flex_1()
            .px_3()
            .py_2()
            .rounded_lg()
            .border_1()
            .border_color(if selected {
                rgb_to_u32(167, 139, 250)
            } else {
                rgb_to_u32(226, 232, 240)
            })
            .bg(if selected {
                rgb_to_u32(245, 243, 255)
            } else {
                rgb_to_u32(255, 255, 255)
            })
            .cursor_pointer()
            .hover(|style| style.bg(rgb_to_u32(248, 250, 252)))
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
                                rgb_to_u32(237, 233, 254)
                            } else {
                                rgb_to_u32(241, 245, 249)
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
                                    .text_color(rgb_to_u32(30, 41, 59))
                                    .child(protocol.label()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb_to_u32(100, 116, 139))
                                    .child(protocol.description()),
                            ),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.protocol = protocol;
                this.error = None;
                cx.notify();
            }))
            .into_any_element()
    }

    pub(super) fn connection_panel(&self) -> Div {
        v_flex()
            .flex_1()
            .h_full()
            .p_3()
            .gap_2()
            .rounded_xl()
            .border_1()
            .border_color(rgb_to_u32(226, 232, 240))
            .bg(rgb_to_u32(255, 255, 255))
            .child(Self::panel_heading(
                IconName::SquareTerminal,
                "连接信息",
                "配置远程主机与登录凭据",
            ))
            .child(Self::field("标题", &self.name))
            .child(Self::field("主机", &self.host))
            .child(Self::field("端口", &self.port))
            .child(Self::field("用户名", &self.username))
            .child(Self::field("密码", &self.password))
    }

    pub(super) fn proxy_panel(&self) -> Div {
        v_flex()
            .flex_1()
            .h_full()
            .p_3()
            .gap_2()
            .rounded_xl()
            .border_1()
            .border_color(rgb_to_u32(226, 232, 240))
            .bg(rgb_to_u32(255, 255, 255))
            .child(Self::panel_heading(
                IconName::Settings2,
                "代理",
                "可选的 SOCKS 或跳板代理配置",
            ))
            .child(Self::field("代理主机", &self.proxy_host))
            .child(Self::field("代理端口", &self.proxy_port))
            .child(Self::field("代理用户名", &self.proxy_username))
            .child(Self::field("代理密码", &self.proxy_password))
    }

    fn panel_heading(icon: IconName, title: &'static str, description: &'static str) -> Div {
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
                    .bg(rgb_to_u32(237, 233, 254))
                    .child(Icon::new(icon).small().text_color(rgb_to_u32(109, 40, 217))),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb_to_u32(30, 41, 59))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb_to_u32(100, 116, 139))
                            .child(description),
                    ),
            )
    }

    pub(super) fn section_option(&self, section: FormSection, cx: &Context<Self>) -> AnyElement {
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
                rgb_to_u32(237, 233, 254)
            } else {
                rgb_to_u32(241, 245, 249)
            })
            .border_1()
            .border_color(if selected {
                rgb_to_u32(196, 181, 253)
            } else {
                rgb_to_u32(241, 245, 249)
            })
            .hover(|style| style.bg(rgb_to_u32(238, 242, 255)))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(section.icon()).small().text_color(if selected {
                        rgb_to_u32(109, 40, 217)
                    } else {
                        rgb_to_u32(100, 116, 139)
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
                                    .text_color(rgb_to_u32(51, 65, 85))
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb_to_u32(100, 116, 139))
                                    .child(section.description()),
                            ),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.section = section;
                this.error = None;
                cx.notify();
            }))
            .into_any_element()
    }
}
