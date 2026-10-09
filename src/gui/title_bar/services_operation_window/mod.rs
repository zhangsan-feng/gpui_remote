use gpui_kit::component::input::InputState;
use gpui_kit::*;

mod core;
mod external;
mod http_proxy_core;
mod http_proxy_ui;
mod port_forward_core;
mod port_forward_rule_window;
mod port_forward_ui;
mod port_test_core;
mod port_test_ui;
mod socks5_proxy_core;
mod socks5_proxy_ui;
mod ssh_server_core;
mod ssh_server_ui;
mod ui;

pub(crate) use external::open_services_window;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ServicesSection {
    Mcp,
    PortForward,
    HttpProxy,
    PortTest,
    SshServer,
    Socks5Proxy,
}

pub struct ServicesOperationWindow {
    mcp_enabled: bool,
    mcp_token_enabled: bool,
    mcp_host: Entity<InputState>,
    mcp_port: Entity<InputState>,
    mcp_token: String,
    mcp_error: Option<String>,
    ssh_enabled: bool,
    ssh_host: Entity<InputState>,
    ssh_port: Entity<InputState>,
    ssh_username: Entity<InputState>,
    ssh_password: Entity<InputState>,
    ssh_error: Option<String>,
    socks5_enabled: bool,
    socks5_host: Entity<InputState>,
    socks5_port: Entity<InputState>,
    socks5_username: Entity<InputState>,
    socks5_password: Entity<InputState>,
    socks5_error: Option<String>,
    http_proxy_enabled: bool,
    http_proxy_host: Entity<InputState>,
    http_proxy_port: Entity<InputState>,
    http_proxy_username: Entity<InputState>,
    http_proxy_password: Entity<InputState>,
    http_proxy_error: Option<String>,
    http_proxy_loading: bool,
    http_proxy_busy: bool,
    port_test_host: Entity<InputState>,
    port_test_port: Entity<InputState>,
    port_test_busy: bool,
    port_test_error: Option<String>,
    port_test_result: Option<crate::application::port_test::PortTestResult>,
    active_section: ServicesSection,
    port_forward_rules: Vec<crate::domain::port_forward::PortForwardRule>,
    port_forward_error: Option<String>,
    port_forward_busy: bool,
    port_forward_delete_confirmation: Option<String>,
}

impl ServicesOperationWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mcp_settings = crate::application::APPLICATION
            .clone()
            .mcp
            .current_settings();
        let ssh_defaults = crate::domain::ssh_server::SshServerSettings::default();
        let ssh_host = Self::input_with_value(ssh_defaults.host.clone(), "监听地址", window, cx);
        let ssh_port =
            Self::input_with_value(ssh_defaults.port.to_string(), "监听端口", window, cx);
        let ssh_username =
            Self::input_with_value(ssh_defaults.username.clone(), "登录用户名", window, cx);
        let ssh_password =
            Self::input_with_value(ssh_defaults.password.clone(), "登录密码", window, cx);
        let ssh_window_handle = window.window_handle();
        let ssh_application = crate::application::APPLICATION.clone();
        let ssh_host_for_load = ssh_host.clone();
        let ssh_port_for_load = ssh_port.clone();
        let ssh_username_for_load = ssh_username.clone();
        let ssh_password_for_load = ssh_password.clone();
        cx.spawn(async move |this, cx| {
            match crate::global_state::run_application(async move {
                ssh_application.ssh_server.current_settings().await
            })
            .await
            {
                Ok(settings) => {
                    let enabled = settings.enabled;
                    let host = settings.host.clone();
                    let port = settings.port.to_string();
                    let username = settings.username.clone();
                    let password = settings.password.clone();
                    let _ = cx.update_window(ssh_window_handle, |_, window, cx| {
                        ssh_host_for_load.update(cx, |input, cx| input.set_value(host, window, cx));
                        ssh_port_for_load.update(cx, |input, cx| input.set_value(port, window, cx));
                        ssh_username_for_load
                            .update(cx, |input, cx| input.set_value(username, window, cx));
                        ssh_password_for_load
                            .update(cx, |input, cx| input.set_value(password, window, cx));
                    });
                    let _ = this.update(cx, |this, cx| {
                        this.ssh_enabled = enabled;
                        this.ssh_error = None;
                        cx.notify();
                    });
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.ssh_error = Some(error);
                        cx.notify();
                    });
                }
            }
        })
        .detach();

        let socks5_defaults = crate::domain::socks5_proxy::Socks5ProxySettings::default();
        let socks5_host =
            Self::input_with_value(socks5_defaults.host.clone(), "监听地址", window, cx);
        let socks5_port =
            Self::input_with_value(socks5_defaults.port.to_string(), "监听端口", window, cx);
        let socks5_username = Self::input_with_value(
            socks5_defaults.username.clone(),
            "用户名（可选）",
            window,
            cx,
        );
        let socks5_password =
            Self::input_with_value(socks5_defaults.password.clone(), "密码（可选）", window, cx);
        let socks5_window_handle = window.window_handle();
        let socks5_application = crate::application::APPLICATION.clone();
        let socks5_host_for_load = socks5_host.clone();
        let socks5_port_for_load = socks5_port.clone();
        let socks5_username_for_load = socks5_username.clone();
        let socks5_password_for_load = socks5_password.clone();
        cx.spawn(async move |this, cx| {
            match crate::global_state::run_application(async move {
                socks5_application.socks5_proxy.current_settings().await
            })
            .await
            {
                Ok(settings) => {
                    let host = settings.host.clone();
                    let port = settings.port.to_string();
                    let username = settings.username.clone();
                    let password = settings.password.clone();
                    let _ = cx.update_window(socks5_window_handle, |_, window, cx| {
                        socks5_host_for_load
                            .update(cx, |input, cx| input.set_value(host, window, cx));
                        socks5_port_for_load
                            .update(cx, |input, cx| input.set_value(port, window, cx));
                        socks5_username_for_load
                            .update(cx, |input, cx| input.set_value(username, window, cx));
                        socks5_password_for_load
                            .update(cx, |input, cx| input.set_value(password, window, cx));
                    });
                    let _ = this.update(cx, |this, cx| {
                        this.socks5_enabled = settings.enabled;
                        this.socks5_error = None;
                        cx.notify();
                    });
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.socks5_error = Some(error);
                        cx.notify();
                    });
                }
            }
        })
        .detach();

        let http_proxy_defaults = crate::domain::http_proxy::HttpProxySettings::default();
        let http_proxy_host =
            Self::input_with_value(http_proxy_defaults.host.clone(), "监听地址", window, cx);
        let http_proxy_port =
            Self::input_with_value(http_proxy_defaults.port.to_string(), "监听端口", window, cx);
        let http_proxy_username = Self::input_with_value(
            http_proxy_defaults.username.clone(),
            "用户名（可选）",
            window,
            cx,
        );
        let http_proxy_password = Self::input_with_value(
            http_proxy_defaults.password.clone(),
            "密码（可选）",
            window,
            cx,
        );
        let http_proxy_window_handle = window.window_handle();
        let http_proxy_application = crate::application::APPLICATION.clone();
        let http_proxy_host_for_load = http_proxy_host.clone();
        let http_proxy_port_for_load = http_proxy_port.clone();
        let http_proxy_username_for_load = http_proxy_username.clone();
        let http_proxy_password_for_load = http_proxy_password.clone();
        cx.spawn(async move |this, cx| {
            match crate::global_state::run_application(async move {
                http_proxy_application.http_proxy.current_settings().await
            })
            .await
            {
                Ok(settings) => {
                    let enabled = settings.enabled;
                    let host = settings.host;
                    let port = settings.port.to_string();
                    let username = settings.username;
                    let password = settings.password;
                    let _ = cx.update_window(http_proxy_window_handle, |_, window, cx| {
                        http_proxy_host_for_load
                            .update(cx, |input, cx| input.set_value(host, window, cx));
                        http_proxy_port_for_load
                            .update(cx, |input, cx| input.set_value(port, window, cx));
                        http_proxy_username_for_load
                            .update(cx, |input, cx| input.set_value(username, window, cx));
                        http_proxy_password_for_load
                            .update(cx, |input, cx| input.set_value(password, window, cx));
                    });
                    let _ = this.update(cx, |this, cx| {
                        this.http_proxy_enabled = enabled;
                        this.http_proxy_error = None;
                        this.http_proxy_loading = false;
                        cx.notify();
                    });
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.http_proxy_error = Some(error);
                        this.http_proxy_loading = false;
                        cx.notify();
                    });
                }
            }
        })
        .detach();

        let mut gui_refresh = crate::data_context::DATA_CONTEXT
            .notice
            .subscribe_gui_refresh();
        cx.spawn(async move |this, cx| {
            let _ = this.update(cx, |this, cx| this.refresh_port_forward_rules(cx));
            while gui_refresh.changed().await.is_ok() {
                gui_refresh.borrow_and_update();
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();

        Self {
            mcp_enabled: mcp_settings.enabled,
            mcp_token_enabled: mcp_settings.token_enabled,
            mcp_host: Self::input_with_value(mcp_settings.host, "监听地址", window, cx),
            mcp_port: Self::input_with_value(mcp_settings.port.to_string(), "监听端口", window, cx),
            mcp_token: mcp_settings.token,
            mcp_error: None,
            ssh_enabled: ssh_defaults.enabled,
            ssh_host,
            ssh_port,
            ssh_username,
            ssh_password,
            ssh_error: None,
            socks5_enabled: socks5_defaults.enabled,
            socks5_host,
            socks5_port,
            socks5_username,
            socks5_password,
            socks5_error: None,
            http_proxy_enabled: http_proxy_defaults.enabled,
            http_proxy_host,
            http_proxy_port,
            http_proxy_username,
            http_proxy_password,
            http_proxy_error: None,
            http_proxy_loading: true,
            http_proxy_busy: false,
            port_test_host: Self::input_with_value(String::new(), "主机名或 IP 地址", window, cx),
            port_test_port: Self::input_with_value(String::new(), "端口", window, cx),
            port_test_busy: false,
            port_test_error: None,
            port_test_result: None,
            active_section: ServicesSection::Mcp,
            port_forward_rules: Vec::new(),
            port_forward_error: None,
            port_forward_busy: true,
            port_forward_delete_confirmation: None,
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

impl Render for ServicesOperationWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(window, cx)
    }
}
