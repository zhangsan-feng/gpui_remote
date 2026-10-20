mod core;
mod external;
mod internal;
mod network_config;
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
    SshTunnel,
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
    proxy_select: Entity<
        SelectState<
            SearchableVec<network_config::ConfigChoice<crate::domain::session::ProxyConfig>>,
        >,
    >,
    tunnel_select: Entity<
        SelectState<
            SearchableVec<
                network_config::ConfigChoice<crate::domain::session::SshReverseTunnelConfig>,
            >,
        >,
    >,
    proxy_choices: Vec<network_config::ConfigChoice<crate::domain::session::ProxyConfig>>,
    tunnel_choices:
        Vec<network_config::ConfigChoice<crate::domain::session::SshReverseTunnelConfig>>,
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
        let ssh_tunnel = profile
            .as_ref()
            .and_then(|profile| profile.ssh_reverse_tunnel.as_ref());
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

        let configs = crate::data_context::DATA_CONTEXT.network_configs();
        let (proxy_choices, proxy_index) =
            network_config::choices(&configs.proxies, proxy.cloned(), "不使用代理");
        let (tunnel_choices, tunnel_index) =
            network_config::choices(&configs.tunnels, ssh_tunnel.cloned(), "不使用 SSH 隧道");
        let proxy_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(proxy_choices.clone()),
                Some(IndexPath::new(proxy_index)),
                window,
                cx,
            )
        });
        let tunnel_select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(tunnel_choices.clone()),
                Some(IndexPath::new(tunnel_index)),
                window,
                cx,
            )
        });
        cx.observe(&proxy_select, |_, _, cx| cx.notify()).detach();
        cx.observe(&tunnel_select, |_, _, cx| cx.notify()).detach();
        let mut updates = crate::data_context::DATA_CONTEXT
            .notice
            .subscribe_gui_refresh();
        cx.spawn(async move |this, cx| {
            while updates.changed().await.is_ok() {
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
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
            proxy_select,
            tunnel_select,
            proxy_choices,
            tunnel_choices,
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_network_choices(window, cx);
        self.render_view(cx)
    }
}
