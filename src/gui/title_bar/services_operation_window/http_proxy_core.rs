use gpui_kit::*;

use crate::domain::http_proxy::HttpProxySettings;

use super::ServicesOperationWindow;

impl ServicesOperationWindow {
    pub(super) fn generate_http_proxy_password(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let password = uuid::Uuid::new_v4().simple().to_string();
        self.http_proxy_password
            .update(cx, |input, cx| input.set_value(password, window, cx));
        self.http_proxy_error = None;
        cx.notify();
    }

    pub(super) fn copy_http_proxy_password(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let password = self.http_proxy_password.read(cx).value().to_string();
        if password.is_empty() {
            self.http_proxy_error = Some("请先设置或随机生成密码".to_owned());
        } else {
            cx.write_to_clipboard(ClipboardItem::new_string(password));
            self.http_proxy_error = None;
        }
        cx.notify();
    }

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
        let restart_started_at = std::time::Instant::now();
        self.http_proxy_busy = true;
        self.http_proxy_error = None;
        cx.notify();

        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.http_proxy.update_settings(settings).await
            })
            .await;
            super::core::keep_restart_feedback_visible_since(restart_started_at).await;
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
