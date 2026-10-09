use gpui_kit::Context;

use crate::{
    data_context::DataSnapshot,
    global_state::{GlobalEvent, read_global_state},
};

use super::Workspace;

impl Workspace {
    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        let event_source = global_state.clone();
        cx.subscribe(&event_source, move |_, _, event, cx| match event {
            GlobalEvent::OpenWorkspaceSession {
                profile_id,
                protocol,
                ip,
                title,
                send_tunnel_proxy_exports,
            } => {
                let application =
                    crate::application::APPLICATION.clone();
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
                    match crate::global_state::run_application(async move {
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
            GlobalEvent::CloseWorkspaceSession { workspace_id } => {
                let application =
                    crate::application::APPLICATION.clone();
                let workspace_id = workspace_id.clone();
                cx.spawn(async move |_this, _cx| {
                    let request_workspace_id = workspace_id.clone();
                    if let Err(error) = crate::global_state::run_application(async move {
                        application.close_session(&request_workspace_id).await
                    }).await {
                        log::debug!("关闭会话失败: workspace_id={workspace_id}, error={error}");
                    }
                })
                .detach();
            }
            GlobalEvent::SelectWorkspaceSession(workspace_id) => {
                let application =
                    crate::application::APPLICATION.clone();
                let workspace_id = workspace_id.clone();
                cx.spawn(async move |_this, _cx| {
                    if let Err(error) = application.sessions.select(workspace_id).await {
                        log::debug!("选择会话失败: {error}");
                    }
                })
                .detach();
            }
            _ => {}
        })
        .detach();

        self.start_data_context_subscription(cx);
    }

    pub(super) fn sync_from_context(&mut self, cx: &mut Context<Self>) {
        let snapshot = crate::data_context::DATA_CONTEXT.workspace_snapshot();
        self.apply_workspace_snapshot(&snapshot, cx);
    }

    fn apply_workspace_snapshot(&mut self, snapshot: &DataSnapshot, cx: &mut Context<Self>) {
        self.workspace
            .update(cx, |workspace, cx| workspace.apply_snapshot(snapshot, cx));
        self.terminal
            .update(cx, |terminal, cx| terminal.apply_snapshot(snapshot, cx));
        self.sftp
            .update(cx, |sftp, cx| sftp.apply_snapshot(snapshot, cx));
        self.select_workspace(snapshot.selected_workspace_id.as_deref(), cx);
    }

    fn start_data_context_subscription(&self, cx: &mut Context<Self>) {
        let data_context = &*crate::data_context::DATA_CONTEXT;
        let mut gui_refresh = crate::data_context::DATA_CONTEXT
            .notice
            .subscribe_gui_refresh();
        cx.spawn(async move |this, cx| {
            while gui_refresh.changed().await.is_ok() {
                gui_refresh.borrow_and_update();
                let snapshot = data_context.workspace_snapshot();
                if this
                    .update(cx, |this, cx| {
                        this.apply_workspace_snapshot(&snapshot, cx);
                    })
                    .is_err()
                {
                    log::debug!("GUI data context subscription stopped: workspace dropped");
                    break;
                }
            }
            log::info!("GUI refresh notification subscription stopped");
        })
        .detach();
    }
}
