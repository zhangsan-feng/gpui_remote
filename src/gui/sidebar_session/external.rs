use crate::{
    application::session::model::SessionSummary,
    domain::session::Protocol,
    global_state::{GlobalEvent, read_global_state},
};
use gpui_kit::Context;

use super::SessionComponent;

impl SessionComponent {
    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_events = read_global_state(cx);
        cx.subscribe(&global_events, |this, _, event, cx| {
            match event {
                GlobalEvent::CreateSession | GlobalEvent::UpdateSession => {}
                GlobalEvent::ThemeColorChanged => {
                    this.refer_item(cx);
                    cx.notify();
                    return;
                }
                _ => return,
            }
            this.refresh_sessions(cx);
        })
        .detach();
    }

    pub(super) fn open_workspace(
        &self,
        session: SessionSummary,
        protocol: Protocol,
        cx: &mut Context<Self>,
    ) {
        self.open_workspace_with_tunnel_proxy(session, protocol, false, cx);
    }

    pub(super) fn open_workspace_with_tunnel_proxy(
        &self,
        session: SessionSummary,
        protocol: Protocol,
        send_tunnel_proxy_exports: bool,
        cx: &mut Context<Self>,
    ) {
        log::debug!(
            "GUI 发送打开会话事件: profile_id={}, protocol={}, host={}, title={}",
            session.id,
            protocol,
            session.host,
            session.name
        );
        read_global_state(cx).update(cx, |_, cx| {
            cx.emit(GlobalEvent::OpenWorkspaceSession {
                profile_id: session.id,
                protocol,
                ip: session.host,
                title: session.name,
                send_tunnel_proxy_exports,
            });
        });
    }
}
