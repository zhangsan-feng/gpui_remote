use gpui_kit::*;

use crate::domain::http_proxy::HttpProxySettings;

use super::ServicesOperationWindow;

impl ServicesOperationWindow {
    pub(super) fn toggle_http_proxy(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_http_proxy_settings(!self.http_proxy_enabled, cx);
    }

    pub(super) fn apply_http_proxy(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_http_proxy_settings(self.http_proxy_enabled, cx);
    }

    fn apply_http_proxy_settings(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.http_proxy_loading || self.http_proxy_busy {
            return;
        }
        let port = match self.http_proxy_port.read(cx).value().trim().parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                self.http_proxy_error = Some("HTTP 代理端口必须是 1-65535 的数字".to_owned());
                cx.notify();
                return;
            }
        };
        let settings = HttpProxySettings {
            enabled,
            host: "0.0.0.0".to_owned(),
            port,
            username: self.http_proxy_username.read(cx).value().to_string(),
            password: self.http_proxy_password.read(cx).value().to_string(),
        };
        self.http_proxy_busy = true;
        self.http_proxy_error = None;
        cx.notify();

        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.http_proxy.update_settings(settings).await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.http_proxy_busy = false;
                match result {
                    Ok(settings) => {
                        this.http_proxy_enabled = settings.enabled;
                        this.http_proxy_error = None;
                    }
                    Err(error) => this.http_proxy_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
