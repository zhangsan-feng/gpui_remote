mod core;
mod external;
mod internal;
mod sftp;
mod ssh;
mod top_session;
mod ui;

use gpui_kit::*;

use crate::data_context::GuiContext;
use crate::domain::session::Protocol;
use sftp::SftpView;
use ssh::TerminalView;
use top_session::WorkspaceSession;

pub struct Workspace {
    gui: GuiContext,
    workspace: Entity<WorkspaceSession>,
    terminal: Entity<TerminalView>,
    sftp: Entity<SftpView>,
    active_protocol: Option<Protocol>,
}

impl Workspace {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        ssh::init(cx);

        let data_context = crate::infrastructure::agent_mcp::start(
            cx.global::<crate::infrastructure::storage::Storage>()
                .session
                .clone(),
        );

        let workspace = cx.new(|cx| WorkspaceSession::new(cx));
        let terminal = cx.new(|cx| TerminalView::new(data_context.gui(), cx));
        let sftp = cx.new(|cx| SftpView::new(data_context.gui(), cx));

        let this = Self {
            gui: data_context.gui(),
            workspace,
            terminal,
            sftp,
            active_protocol: None,
        };
        this.start_status_watchers(cx);
        this.start_subscribe(cx);
        this.refresh_session_statuses(cx);
        this
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(cx)
    }
}
