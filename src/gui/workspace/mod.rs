mod core;
mod external;
mod internal;
mod sftp;
mod ssh;
mod top_session;
mod ui;

use gpui_kit::*;

use crate::application::ApplicationContext;
use crate::domain::session::Protocol;
use sftp::SftpView;
use ssh::TerminalView;
use top_session::WorkspaceSession;

pub struct Workspace {
    application: ApplicationContext,
    workspace: Entity<WorkspaceSession>,
    terminal: Entity<TerminalView>,
    sftp: Entity<SftpView>,
    active_protocol: Option<Protocol>,
}

impl Workspace {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        ssh::init(cx);
        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        let workspace = cx.new(|cx| WorkspaceSession::new(cx));
        let terminal = cx.new(TerminalView::new);
        let sftp = cx.new(SftpView::new);

        let this = Self {
            application,
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
