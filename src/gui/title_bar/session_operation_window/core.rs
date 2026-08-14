use gpui::*;

use crate::domain::session::{NewSession, Protocol, ProxyConfig};

use super::{ConnectionProtocol, SessionOperationWindow};

impl ConnectionProtocol {
    pub(super) const ALL: [Self; 2] = [Self::Ssh, Self::Sftp];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Sftp => "SFTP",
            Self::Telnet => "TELNET",
        }
    }
}

impl SessionOperationWindow {
    pub(super) fn draft(&self, cx: &App) -> Result<NewSession, String> {
        let proxy_host = self.proxy_host.read(cx).value().trim().to_owned();
        let proxy = if proxy_host.is_empty() {
            None
        } else {
            Some(ProxyConfig {
                host: proxy_host,
                port: parse_port(self.proxy_port.read(cx).value().as_ref(), "代理")?,
                username: self.proxy_username.read(cx).value().trim().to_owned(),
                password: self.proxy_password.read(cx).value().to_string(),
            })
        };
        let protocol = match self.protocol {
            ConnectionProtocol::Ssh => Protocol::Ssh,
            ConnectionProtocol::Sftp => Protocol::Sftp,
            ConnectionProtocol::Telnet => return Err("暂不支持 TELNET 协议".to_owned()),
        };
        let draft = NewSession {
            protocol,
            name: self.name.read(cx).value().trim().to_owned(),
            host: self.host.read(cx).value().trim().to_owned(),
            port: parse_port(self.port.read(cx).value().as_ref(), "连接")?,
            username: self.username.read(cx).value().trim().to_owned(),
            password: self.password.read(cx).value().to_string(),
            private_key_path: self.private_key_path.clone(),
            proxy,
        };

        draft.validate().map_err(str::to_owned)?;
        Ok(draft)
    }

    pub(super) fn choose_private_key(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("选择 SSH 私钥".into()),
        });
        let this = cx.weak_entity();
        window
            .spawn(cx, async move |cx| {
                let paths = match receiver.await {
                    Ok(Ok(Some(paths))) => paths,
                    _ => return Ok::<(), anyhow::Error>(()),
                };
                let Some(path) = paths.into_iter().next() else {
                    return Ok(());
                };
                let path = path.to_string_lossy().into_owned();
                cx.update(|_, cx| {
                    let _ = this.update(cx, |this, cx| {
                        this.private_key_path = Some(path);
                        cx.notify();
                    });
                })?;
                Ok(())
            })
            .detach();
    }

    pub(super) fn clear_private_key(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.private_key_path = None;
        cx.notify();
    }
}

fn parse_port(value: &str, label: &str) -> Result<u16, String> {
    value
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("{label}端口格式不正确"))
}
