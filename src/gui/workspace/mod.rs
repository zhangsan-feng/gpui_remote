mod core;
mod external;
mod internal;
mod sftp;
mod ssh;
mod top_session;
mod ui;

use gpui_kit::*;

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
        self.render_view(cx)
    }
}
