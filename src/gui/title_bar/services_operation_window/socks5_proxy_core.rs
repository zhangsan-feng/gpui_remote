use gpui_kit::*;

use crate::domain::socks5_proxy::Socks5ProxySettings;

use super::ServicesOperationWindow;

impl ServicesOperationWindow {
    pub(super) fn toggle_socks5_proxy(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.socks5_enabled = !self.socks5_enabled;
        cx.notify();
    }

    pub(super) fn apply_socks5_proxy(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let port = match self.socks5_port.read(cx).value().trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                self.socks5_error = Some("SOCKS5 端口必须是 1-65535 的数字".to_owned());
                cx.notify();
                return;
            }
        };
        let settings = Socks5ProxySettings {
            enabled: self.socks5_enabled,
            host: self.socks5_host.read(cx).value().to_string(),
            port,
            username: self.socks5_username.read(cx).value().to_string(),
            password: self.socks5_password.read(cx).value().to_string(),
        };
        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.socks5_proxy.update_settings(settings).await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(settings) => {
                        this.socks5_enabled = settings.enabled;
                        this.socks5_error = None;
                    }
                    Err(error) => this.socks5_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
