mod core;
mod external;
mod internal;
mod sftp;
mod ssh;
mod top_session;
mod ui;

use crate::event_bus::{EVENT_BUS, gui_event::GuiTarget};
use gpui_kit::*;

use crate::domain::session::Protocol;
use sftp::SftpView;
use ssh::TerminalView;
use top_session::WorkspaceSession;

pub struct Workspace {
    gui_event_task: Option<Task<()>>,
    workspace: Entity<WorkspaceSession>,
    terminal: Entity<TerminalView>,
    sftp: Entity<SftpView>,
    active_protocol: Option<Protocol>,
}

impl Workspace {
    pub fn new(scope: EntityId, _: &mut Window, cx: &mut Context<Self>) -> Self {
        ssh::init(cx);
        let workspace = cx.new(|cx| WorkspaceSession::new(scope, cx));
        let terminal = cx.new(TerminalView::new);
        let sftp = cx.new(SftpView::new);

        let mut this = Self {
            gui_event_task: None,
            workspace,
            terminal,
            sftp,
            active_protocol: None,
        };
        this.start_subscribe(scope, cx);
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
    fn start_subscribe(&mut self, scope: EntityId, cx: &mut Context<Self>) {
        let mut events = EVENT_BUS.gui_event.subscribe(scope, GuiTarget::Workspace);
        self.gui_event_task = Some(cx.spawn(async move |this, cx| {
            while let Some(event) = events.receiver.recv().await {
                if this
                    .update(cx, |this, cx| this.handle_gui_event(&event, cx))
                    .is_err()
                {
                    break;
                }
            }
        }));
        self.start_data_context_subscription(cx);
    }

    fn start_data_context_subscription(&self, cx: &mut Context<Self>) {
        let mut refresh = crate::event_bus::EVENT_BUS
            .refresh_gui
            .subscribe(crate::event_bus::refresh_gui::GuiComponent::Workspace);
        cx.spawn(async move |this, cx| {
            while refresh.changed().await.is_ok() {
                let scope = refresh.take_pending();
                if this
                    .update(cx, |this, cx| {
                        if scope.all {
                            let snapshot = crate::data_context::DATA_CONTEXT.workspace_snapshot();
                            this.workspace.update(cx, |workspace, cx| {
                                workspace.apply_snapshot(&snapshot, cx)
                            });
                            this.select_workspace(snapshot.selected_workspace_id.as_deref(), cx);
                        } else {
                            this.workspace.update(cx, |workspace, cx| {
                                for workspace_id in scope.workspace_ids {
                                    workspace.refresh_session_from_context(&workspace_id, cx);
                                }
                            });
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
}
