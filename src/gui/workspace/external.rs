use gpui_kit::{AppContext, Context};

use crate::{
    application::ApplicationContext,
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
                let profile = profile.clone();
                let global_state = global_state.clone();
                log::debug!(
                    "GUI 打开会话事件收到: profile_id={profile_id}, protocol={protocol}, host={ip}, title={title}"
                );
                cx.spawn(async move |_this, cx| {
                    match application
                        .open_session(profile_id.clone(), protocol, ip, title)
                        .await
                    {
                        Ok(workspace_id) => {
                            log::debug!(
                                "GUI 打开会话完成，发布 workspace opened: workspace_id={workspace_id}, profile_id={profile_id}, protocol={protocol}"
                            );
                            global_state.update(cx, |_, cx| {
                                cx.emit(GlobalEvent::WorkspaceSessionOpened(workspace_id, profile));
                            });
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
                this.select_workspace(workspace_id.as_deref(), cx);
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
            _ => {}
        })
        .detach();
    }
}
