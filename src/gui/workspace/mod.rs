mod core;
mod external;
mod internal;
mod sftp;
mod ssh;
mod top_session;
mod ui;

use crate::global_state::{GlobalEvent, read_global_state};
use gpui_kit::*;
use std::time::Instant;

use crate::domain::session::Protocol;
use sftp::SftpView;
use ssh::TerminalView;
use top_session::WorkspaceSession;

pub struct Workspace {
    workspace: Entity<WorkspaceSession>,
    terminal: Entity<TerminalView>,
    sftp: Entity<SftpView>,
    active_protocol: Option<Protocol>,
}

impl Workspace {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        ssh::init(cx);
        let workspace = cx.new(|cx| WorkspaceSession::new(cx));
        let terminal = cx.new(TerminalView::new);
        let sftp = cx.new(SftpView::new);

        let mut this = Self {
            workspace,
            terminal,
            sftp,
            active_protocol: None,
        };
        this.start_subscribe(cx);
        this.sync_from_context(cx);
        this
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.render_view(cx);
        div().size_full().child(content)
    }
}

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

    fn start_data_context_subscription(&self, cx: &mut Context<Self>) {
        let data_context = &*crate::data_context::DATA_CONTEXT;
        let mut gui_refresh = crate::event_bus::EVENT_BUS
            .refresh_gui
            .subscribe(crate::event_bus::refresh_gui::GuiComponent::Workspace);
        cx.spawn(async move |this, cx| {
            while gui_refresh.changed().await.is_ok() {
                let refresh_sequence = *gui_refresh.borrow_and_update();
                let process_id = std::process::id();
                log::debug!(
                    "GUI refresh notification received: process_id={process_id}, refresh_sequence={refresh_sequence}"
                );

                let snapshot_started = Instant::now();
                let snapshot = data_context.workspace_snapshot();
                log::debug!(
                    "GUI refresh snapshot cloned: refresh_sequence={refresh_sequence}, revision={}, workspaces={}, elapsed_us={}",
                    snapshot.revision,
                    snapshot.workspaces.len(),
                    snapshot_started.elapsed().as_micros()
                );

                let dispatch_started = Instant::now();
                log::debug!(
                    "GUI refresh entity update requested: refresh_sequence={refresh_sequence}, revision={}",
                    snapshot.revision
                );
                let update_result = this.update(cx, |this, cx| {
                    log::debug!(
                        "GUI refresh entity update entered: refresh_sequence={refresh_sequence}, revision={}",
                        snapshot.revision
                    );
                    this.workspace.update(cx, |workspace, cx| workspace.apply_snapshot(&snapshot, cx));
                    this.select_workspace(snapshot.selected_workspace_id.as_deref(), cx);
                    log::debug!(
                        "GUI refresh entity update applied: refresh_sequence={refresh_sequence}, revision={}",
                        snapshot.revision
                    );
                });
                if update_result.is_err() {
                    log::debug!("GUI data context subscription stopped: workspace dropped");
                    break;
                }
                log::debug!(
                    "GUI refresh entity update completed: refresh_sequence={refresh_sequence}, revision={}, elapsed_us={}",
                    snapshot.revision,
                    dispatch_started.elapsed().as_micros()
                );
            }
            log::debug!("GUI refresh notification subscription stopped");
        })
        .detach();
    }
}
