use gpui_kit::Context;

use crate::global_state::{GlobalEvent, read_global_state};

use super::Workspace;

impl Workspace {
    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        cx.subscribe(&global_state, |this, _, event, cx| match event {
            GlobalEvent::OpenWorkspaceSession(workspace_id, profile) => {
                let gui = this.gui.clone();
                let workspace_id = workspace_id.clone();
                let profile_id = profile.id.clone();
                let protocol = profile.protocol;
                let ip = profile.host.clone();
                let title = profile.name.clone();
                cx.spawn(async move |_this, _cx| {
                    if let Err(error) = gui.open_session(profile_id, protocol, ip, title).await {
                        log::warn!(
                            "打开 {} 会话失败: workspace_id={workspace_id}, error={error}",
                            protocol
                        );
                    }
                })
                .detach();
            }
            GlobalEvent::CloseWorkspaceSession { workspace_id } => {
                let gui = this.gui.clone();
                let workspace_id = workspace_id.clone();
                cx.spawn(async move |_this, _cx| {
                    if let Err(error) = gui.close_session(workspace_id.clone()).await {
                        log::debug!("关闭会话失败: workspace_id={workspace_id}, error={error}");
                    }
                })
                .detach();
            }
            GlobalEvent::SelectWorkspaceSession(workspace_id) => {
                this.select_workspace(workspace_id.as_deref(), cx);
                let gui = this.gui.clone();
                let workspace_id = workspace_id.clone();
                cx.spawn(async move |_this, _cx| {
                    if let Err(error) = gui.select_session(workspace_id).await {
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
