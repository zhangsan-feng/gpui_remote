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
        self.apply_socks5_proxy_settings(!self.socks5_enabled, cx);
    }

    pub(super) fn apply_socks5_proxy(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_socks5_proxy_settings(self.socks5_enabled, cx);
    }

    fn apply_socks5_proxy_settings(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.socks5_loading || self.socks5_busy {
            return;
        }
        let port = match self.socks5_port.read(cx).value().trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                self.socks5_error = Some("SOCKS5 端口必须是 1-65535 的数字".to_owned());
                cx.notify();
                return;
            }
        };
        let settings = Socks5ProxySettings {
            enabled,
            host: "0.0.0.0".to_owned(),
            port,
            username: self.socks5_username.read(cx).value().to_string(),
            password: self.socks5_password.read(cx).value().to_string(),
        };
        self.socks5_busy = true;
        self.socks5_error = None;
        cx.notify();

        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.socks5_proxy.update_settings(settings).await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.socks5_busy = false;
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
