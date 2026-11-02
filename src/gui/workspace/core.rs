use gpui_kit::Context;

use crate::domain::session::Protocol;

use super::Workspace;

impl Workspace {
    pub(super) fn protocol_for_workspace(
        &self,
        workspace_id: Option<&str>,
        cx: &Context<Self>,
    ) -> Option<Protocol> {
        let workspace_id = workspace_id?;
        self.workspace
            .read(cx)
            .sessions()
            .iter()
            .find(|opened| opened.id == workspace_id)
            .map(|opened| opened.profile.protocol)
    }
}

impl Workspace {
    pub(super) fn handle_gui_event(
        &mut self,
        event: &crate::event_bus::gui_event::GuiEvent,
        cx: &mut Context<Self>,
    ) {
        use crate::event_bus::gui_event::GuiEvent;
        match event {
            GuiEvent::OpenWorkspaceSession {
                profile_id,
                protocol,
                ip,
                title,
                send_tunnel_proxy_exports,
            } => {
                let application = crate::application::APPLICATION.clone();
                let profile_id = profile_id.clone();
                let protocol = *protocol;
                let ip = ip.clone();
                let title = title.clone();
                let send_tunnel_proxy_exports = *send_tunnel_proxy_exports;
                log::debug!(
                    "GUI 打开会话事件收到: profile_id={profile_id}, protocol={protocol}, host={ip}, title={title}"
                );
                cx.spawn(async move |_this, _cx| {
                    let request_profile_id = profile_id.clone();
                    match crate::application::run_application(async move {
                        application
                            .open_session(
                                request_profile_id,
                                protocol,
                                ip,
                                title,
                                send_tunnel_proxy_exports,
                            )
                            .await
                    }).await
                    {
                        Ok(workspace_id) => {
                            log::debug!(
                                "GUI 打开会话完成，等待 data context 同步: workspace_id={workspace_id}, profile_id={profile_id}, protocol={protocol}"
                            );
                        }
                        Err(error) => {
                            log::warn!(
                                "打开 {} 会话失败: profile_id={profile_id}, error={error}",
                                protocol
                            );
                        }
                    }
                })
                .detach();
            }
            GuiEvent::CloseWorkspaceSession { workspace_id } => {
                let application = crate::application::APPLICATION.clone();
                let workspace_id = workspace_id.clone();
                cx.spawn(async move |_this, _cx| {
                    let request_workspace_id = workspace_id.clone();
                    if let Err(error) = crate::application::run_application(async move {
                        application.close_session(&request_workspace_id).await
                    })
                    .await
                    {
                        log::debug!("关闭会话失败: workspace_id={workspace_id}, error={error}");
                    }
                })
                .detach();
            }
            GuiEvent::SelectWorkspaceSession(workspace_id) => {
                let application = crate::application::APPLICATION.clone();
                let workspace_id = workspace_id.clone();
                cx.spawn(async move |_this, _cx| {
                    if let Err(error) = crate::application::run_application(async move {
                        application.sessions.select(workspace_id).await
                    })
                    .await
                    {
                        log::debug!("选择会话失败: {error}");
                    }
                })
                .detach();
            }
            _ => {}
        }
    }
}
