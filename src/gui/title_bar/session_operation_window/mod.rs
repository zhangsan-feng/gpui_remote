mod core;
mod external;
mod internal;
mod ui;

use gpui_kit::component::{
    IndexPath,
    input::InputState,
    select::{SearchableVec, SelectEvent, SelectState},
};
use gpui_kit::*;

use crate::domain::session::SessionProfile;

pub(crate) use external::{open_edit_session_window, open_new_session_window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionProtocol {
    SshAndSftp,
    Mysql,
    Pgsql,
    Redis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FormSection {
    Connection,
    Proxy,
}

#[derive(Clone)]
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
    private_key_path: Option<String>,
    protocol_select: Entity<SelectState<SearchableVec<ConnectionProtocol>>>,
    proxy_host: Entity<InputState>,
    proxy_port: Entity<InputState>,
    proxy_username: Entity<InputState>,
    proxy_password: Entity<InputState>,
    error: Option<String>,
}

impl ConnectionProtocol {
    fn from_label(label: &str) -> Self {
        match label.to_ascii_uppercase().as_str() {
            "MYSQL" => Self::Mysql,
            "PGSQL" | "POSTGRES" | "POSTGRESQL" => Self::Pgsql,
            "REDIS" => Self::Redis,
            _ => Self::SshAndSftp,
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
            .map(|profile| ConnectionProtocol::from_label(profile.connection_protocol.as_str()))
            .unwrap_or(ConnectionProtocol::SshAndSftp);
        let proxy = profile.as_ref().and_then(|profile| profile.proxy.as_ref());
        let protocol_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(ConnectionProtocol::ALL.to_vec()),
                Some(IndexPath::new(protocol.index())),
                window,
                cx,
            )
        });
        cx.subscribe(
            &protocol_select,
            |this, _, event: &SelectEvent<SearchableVec<ConnectionProtocol>>, cx| {
                if let SelectEvent::Confirm(Some(protocol)) = event {
                    this.select_protocol(*protocol, cx);
                }
            },
        )
        .detach();

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
            private_key_path: profile.as_ref().and_then(|p| p.private_key_path.clone()),
            protocol_select,
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
        self.render_view(cx)
    }
}
