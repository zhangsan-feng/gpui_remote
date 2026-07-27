use gpui::*;

use crate::{
    domain::session::{NewSession, ProxyConfig, SessionProfile},
    global_state::{GlobalEvent, read_global_state},
    infrastructure::storage::Storage,
};

use super::{ConnectionProtocol, SessionFormMode, SessionOperationWindow};

impl ConnectionProtocol {
    pub(super) const ALL: [Self; 3] = [Self::Ssh, Self::Sftp, Self::Telnet];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Sftp => "SFTP",
            Self::Telnet => "TELNET",
        }
    }
}

impl SessionOperationWindow {
    fn draft(&self, cx: &App) -> Result<NewSession, String> {
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
        let draft = NewSession {
            protocol: self.protocol.label().to_owned(),
            name: self.name.read(cx).value().trim().to_owned(),
            host: self.host.read(cx).value().trim().to_owned(),
            port: parse_port(self.port.read(cx).value().as_ref(), "连接")?,
            username: self.username.read(cx).value().trim().to_owned(),
            password: self.password.read(cx).value().to_string(),
            proxy,
        };

        draft.validate().map_err(str::to_owned)?;
        Ok(draft)
    }

    pub(super) fn submit(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let draft = match self.draft(cx) {
            Ok(draft) => draft,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };

        let result = match &self.mode {
            SessionFormMode::Create => cx
                .global::<Storage>()
                .session
                .insert(draft)
                .map(|_| GlobalEvent::CreateSession),
            SessionFormMode::Edit { id } => cx
                .global::<Storage>()
                .session
                .update(id, draft)
                .map(|_| GlobalEvent::UpdateSession),
        };

        match result {
            Ok(event) => {
                read_global_state(cx).update(cx, |_, cx| {
                    cx.emit(event);
                });
                window.remove_window();
            }
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }

    pub(super) fn cancel(&mut self, _: &ClickEvent, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }
}

pub(crate) fn open_new_session_window(cx: &mut App) {
    open_session_window(None, cx);
}

pub(crate) fn open_edit_session_window<T: 'static>(
    profile: SessionProfile,
    _session_list: Entity<T>,
    cx: &mut App,
) {
    open_session_window(Some(profile), cx);
}

fn open_session_window(profile: Option<SessionProfile>, cx: &mut App) {
    let editing = profile.is_some();
    let window_size = size(px(680.), px(500.));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(window_size, cx)),
        window_min_size: Some(window_size),
        titlebar: Some(TitlebarOptions {
            title: Some(
                if editing {
                    "编辑远程会话"
                } else {
                    "新建远程会话"
                }
                .into(),
            ),
            appears_transparent: false,
            traffic_light_position: None,
        }),
        kind: WindowKind::Dialog,
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };

    let _ = cx.open_window(options, move |window, cx| {
        let form = match profile {
            Some(profile) => cx.new(|cx| SessionOperationWindow::edit(profile, window, cx)),
            None => cx.new(|cx| SessionOperationWindow::new(window, cx)),
        };
        cx.new(|cx| gpui_component::Root::new(form, window, cx))
    });
}

fn parse_port(value: &str, label: &str) -> Result<u16, String> {
    value
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("{label}端口格式不正确"))
}
