mod core;
mod ui;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::InputState,
    v_flex,
};

use crate::{component::color::rgb_to_u32, domain::session::SessionProfile};

pub(crate) use core::{open_edit_session_window, open_new_session_window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionProtocol {
    Ssh,
    Sftp,
    Telnet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FormSection {
    Connection,
    Proxy,
}

enum SessionFormMode {
    Create,
    Edit { id: String },
}

pub struct SessionOperationWindow {
    mode: SessionFormMode,
    protocol: ConnectionProtocol,
    section: FormSection,
    name: Entity<InputState>,
    host: Entity<InputState>,
    port: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    proxy_host: Entity<InputState>,
    proxy_port: Entity<InputState>,
    proxy_username: Entity<InputState>,
    proxy_password: Entity<InputState>,
    error: Option<String>,
}

impl ConnectionProtocol {
    fn from_label(label: &str) -> Self {
        match label {
            "SFTP" => Self::Sftp,
            "TELNET" => Self::Telnet,
            _ => Self::Ssh,
        }
    }
}

impl SessionOperationWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::from_profile(None, SessionFormMode::Create, window, cx)
    }

    fn edit(profile: SessionProfile, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mode = SessionFormMode::Edit {
            id: profile.id.clone(),
        };
        Self::from_profile(Some(profile), mode, window, cx)
    }

    fn from_profile(
        profile: Option<SessionProfile>,
        mode: SessionFormMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let protocol = profile
            .as_ref()
            .map(|profile| ConnectionProtocol::from_label(&profile.protocol))
            .unwrap_or(ConnectionProtocol::Ssh);
        let proxy = profile.as_ref().and_then(|profile| profile.proxy.as_ref());

        Self {
            mode,
            protocol,
            section: FormSection::Connection,
            name: Self::input_with_value(
                profile.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
                "可选，留空不显示",
                window,
                cx,
            ),
            host: Self::input_with_value(
                profile.as_ref().map(|p| p.host.clone()).unwrap_or_default(),
                "主机名或 IP 地址",
                window,
                cx,
            ),
            port: Self::input_with_value(
                profile
                    .as_ref()
                    .map(|p| p.port.to_string())
                    .unwrap_or_else(|| "22".into()),
                "连接端口",
                window,
                cx,
            ),
            username: Self::input_with_value(
                profile
                    .as_ref()
                    .map(|p| p.username.clone())
                    .unwrap_or_default(),
                "登录用户名",
                window,
                cx,
            ),
            password: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("密码（可选）")
                    .default_value(
                        profile
                            .as_ref()
                            .map(|p| p.password.clone())
                            .unwrap_or_default(),
                    )
                    .masked(true)
            }),
            proxy_host: Self::input_with_value(
                proxy.map(|p| p.host.clone()).unwrap_or_default(),
                "留空表示直连",
                window,
                cx,
            ),
            proxy_port: Self::input_with_value(
                proxy
                    .map(|p| p.port.to_string())
                    .unwrap_or_else(|| "1080".into()),
                "代理端口",
                window,
                cx,
            ),
            proxy_username: Self::input_with_value(
                proxy.map(|p| p.username.clone()).unwrap_or_default(),
                "代理用户名（可选）",
                window,
                cx,
            ),
            proxy_password: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("代理密码（可选）")
                    .default_value(proxy.map(|p| p.password.clone()).unwrap_or_default())
                    .masked(true)
            }),
            error: None,
        }
    }

    fn input_with_value(
        value: String,
        placeholder: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(value)
        })
    }
}

impl Render for SessionOperationWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .p_2()
            .size_full()
            .bg(rgb_to_u32(248, 250, 252))
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
                            .border_color(rgb_to_u32(226, 232, 240))
                            .bg(rgb_to_u32(241, 245, 249))
                            .children(
                                FormSection::ALL.map(|section| self.section_option(section, cx)),
                            ),
                    )
                    .child(match self.section {
                        FormSection::Connection => self.connection_panel(),
                        FormSection::Proxy => self.proxy_panel(),
                    }),
            )
            .when_some(self.error.clone(), |this, error| {
                this.child(
                    h_flex()
                        .p_2()
                        .gap_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb_to_u32(254, 202, 202))
                        .bg(rgb_to_u32(254, 242, 242))
                        .text_color(rgb_to_u32(185, 28, 28))
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
                    .border_color(rgb_to_u32(226, 232, 240))
                    .bg(rgb_to_u32(255, 255, 255))
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
