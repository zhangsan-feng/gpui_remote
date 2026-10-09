use gpui_kit::*;

use crate::domain::ssh_server::SshServerSettings;

use super::ServicesOperationWindow;

impl ServicesOperationWindow {
    pub(super) fn toggle_ssh_enabled(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ssh_enabled = !self.ssh_enabled;
        cx.notify();
    }

    pub(super) fn generate_ssh_password(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let password = uuid::Uuid::new_v4().simple().to_string();
        self.ssh_password
            .update(cx, |input, cx| input.set_value(password, window, cx));
        self.ssh_error = None;
        cx.notify();
    }

    pub(super) fn copy_ssh_password(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let password = self.ssh_password.read(cx).value().to_string();
        if password.is_empty() {
            self.ssh_error = Some("请先设置或随机生成密码".to_owned());
        } else {
            cx.write_to_clipboard(ClipboardItem::new_string(password));
            self.ssh_error = None;
        }
        cx.notify();
    }

    pub(super) fn apply_ssh_settings(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let port = match self.ssh_port.read(cx).value().trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                self.ssh_error = Some("SSH 端口必须是 1-65535 的数字".to_owned());
                cx.notify();
                return;
            }
        };
        let settings = SshServerSettings {
            enabled: self.ssh_enabled,
            host: self.ssh_host.read(cx).value().to_string(),
            port,
            username: self.ssh_username.read(cx).value().to_string(),
            password: self.ssh_password.read(cx).value().to_string(),
        };
        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application
                    .ssh_server
                    .update_settings(settings, application.sftp_server.clone())
                    .await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(settings) => {
                        this.ssh_enabled = settings.enabled;
                        this.ssh_error = None;
                    }
                    Err(error) => this.ssh_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
