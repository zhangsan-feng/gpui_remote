use crate::{
    application::session::model::SessionSummary,
    domain::session::Protocol,
    event_bus::{EVENT_BUS, gui_event::GuiEvent},
};
use gpui_kit::Context;

use super::SessionComponent;

impl SessionComponent {
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
        _cx: &mut Context<Self>,
    ) {
        log::debug!(
            "GUI 发送打开会话事件: profile_id={}, protocol={}, host={}, title={}",
            session.id,
            protocol,
            session.host,
            session.name
        );
        EVENT_BUS.gui_event.send(
            self.gui_scope,
            GuiEvent::OpenWorkspaceSession {
                profile_id: session.id,
                protocol,
                ip: session.host,
                title: session.name,
                send_tunnel_proxy_exports,
            },
        );
    }
}
