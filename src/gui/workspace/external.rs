use gpui_kit::{AppContext, Context};

use crate::{
    application::{ApplicationContext, ApplicationEvent},
    global_state::{GlobalEvent, read_global_state},
};

use super::Workspace;

impl Workspace {
    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        let event_source = global_state.clone();
        cx.subscribe(&event_source, move |this, _, event, cx| match event {
            GlobalEvent::OpenWorkspaceSession(profile) => {
                let application =
                    cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
                let profile_id = profile.id.clone();
                let protocol = profile.protocol;
                let ip = profile.host.clone();
                let title = profile.name.clone();
                log::debug!(
                    "GUI 打开会话事件收到: profile_id={profile_id}, protocol={protocol}, host={ip}, title={title}"
                );
                cx.spawn(async move |_this, _cx| {
                    match application
                        .open_session(profile_id.clone(), protocol, ip, title)
                        .await
                    {
                        Ok(workspace_id) => {
                            log::debug!(
                                "GUI 打开会话完成，等待 application event 同步: workspace_id={workspace_id}, profile_id={profile_id}, protocol={protocol}"
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
            GlobalEvent::CloseWorkspaceSession { workspace_id } => {
                let application =
                    cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
                let workspace_id = workspace_id.clone();
                cx.spawn(async move |_this, _cx| {
                    if let Err(error) = application.close_session(&workspace_id).await {
                        log::debug!("关闭会话失败: workspace_id={workspace_id}, error={error}");
                    }
                })
                .detach();
            }
            GlobalEvent::SelectWorkspaceSession(workspace_id) => {
                let application =
                    cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
                let workspace_id = workspace_id.clone();
                cx.spawn(async move |_this, _cx| {
                    if let Err(error) = application.select_session(workspace_id).await {
                        log::debug!("选择会话失败: {error}");
                    }
                })
                .detach();
            }
            GlobalEvent::WorkspaceSessionSelected(workspace_id) => {
                this.select_workspace(workspace_id.as_deref(), cx);
                this.refresh_session_statuses(cx);
            }
            GlobalEvent::WorkspaceSessionOpened(..) | GlobalEvent::WorkspaceSessionClosed { .. } => {
                this.refresh_session_statuses(cx);
            }
            _ => {}
        })
        .detach();

        self.start_application_event_forwarder(cx);
    }

    fn start_application_event_forwarder(&self, cx: &mut Context<Self>) {
        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        let mut application_events = application.subscribe();
        let global_state = read_global_state(cx);
        cx.spawn(async move |this, cx| {
            loop {
                let event = match application_events.recv().await {
                    Ok(event) => event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        log::warn!(
                            "GUI application event receiver lagged: skipped={count}"
                        );
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        log::info!(
                            "GUI application event forwarder stopped: reason=application_events_closed"
                        );
                        break;
                    }
                };
                let event_name = application_event_name(&event);
                log::debug!("GUI application event received: event={event_name}");
                let global_event = map_application_event(event);
                if this
                    .update(cx, |_, cx| {
                        global_state.update(cx, |_, cx| cx.emit(global_event));
                    })
                    .is_err()
                {
                    log::debug!(
                        "GUI application event forwarder stopped: reason=workspace_dropped"
                    );
                    break;
                }
                log::debug!("GUI application event forwarded: event={event_name}");
            }
        })
        .detach();
    }
}

fn map_application_event(event: ApplicationEvent) -> GlobalEvent {
    match event {
        ApplicationEvent::SessionOpened {
            workspace_id,
            profile,
        } => GlobalEvent::WorkspaceSessionOpened(workspace_id, profile),
        ApplicationEvent::SessionClosed { workspace_id } => {
            GlobalEvent::WorkspaceSessionClosed { workspace_id }
        }
        ApplicationEvent::SessionSelected { workspace_id } => {
            GlobalEvent::WorkspaceSessionSelected(workspace_id)
        }
    }
}

fn application_event_name(event: &ApplicationEvent) -> &'static str {
    match event {
        ApplicationEvent::SessionOpened { .. } => "session_opened",
        ApplicationEvent::SessionClosed { .. } => "session_closed",
        ApplicationEvent::SessionSelected { .. } => "session_selected",
    }
}
