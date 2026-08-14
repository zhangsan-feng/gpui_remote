mod agent_mcp;
mod core;
mod external;
mod internal;
mod sftp;
mod ssh;
mod top_session;
mod ui;

use gpui::*;

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

        let (agent_mcp_client, agent_mcp_receiver) =
            crate::application::agent_mcp::agent_mcp_channel();
        crate::infrastructure::agent_mcp::start(agent_mcp_client);

        let workspace = cx.new(|cx| WorkspaceSession::new(cx));
        let terminal = cx.new(TerminalView::new);
        let sftp = cx.new(SftpView::new);

        let this = Self {
            workspace,
            terminal,
            sftp,
            active_protocol: None,
        };
        this.start_status_watchers(cx);
        this.start_subscribe(cx);
        this.start_agent_mcp(agent_mcp_receiver, cx);
        this.refresh_session_statuses(cx);
        this
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(cx)
    }
}
