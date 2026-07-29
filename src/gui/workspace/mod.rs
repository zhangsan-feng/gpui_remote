mod agent_mcp;
mod session;
mod sftp;
mod terminal;
mod ui;

use gpui::*;
use gpui_component::{ActiveTheme, v_flex};

use crate::component::theme;
use session::{WorkspaceSession, terminal_statuses};
use terminal::TerminalView;
use ui::render_empty_workspace;

pub struct Workspace {
    workspace: Entity<WorkspaceSession>,
    terminal: Entity<TerminalView>,
}

impl Workspace {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        terminal::init(cx);

        let (agent_mcp_client, agent_mcp_receiver) =
            crate::application::agent_mcp::agent_mcp_channel();
        crate::infrastructure::agent_mcp::start(agent_mcp_client);

        let workspace = cx.new(|cx| WorkspaceSession::new(cx));
        let terminal = cx.new(TerminalView::new);

        let terminal_updates = terminal.read(cx).status_updates();
        cx.spawn(async move |this, cx| {
            loop {
                terminal_updates.notified().await;
                if this
                    .update(cx, |this, cx| this.refresh_session_statuses(cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        let this = Self {
            workspace,
            terminal,
        };
        this.start_agent_mcp(agent_mcp_receiver, cx);
        cx.observe(&this.workspace, |this, _, cx| {
            this.sync_selected_terminal(cx);
            cx.notify();
        })
        .detach();
        this.sync_selected_terminal(cx);
        this.refresh_session_statuses(cx);
        this
    }

    fn sync_selected_terminal(&self, cx: &mut Context<Self>) {
        let selected_id = self.workspace.read(cx).selected_id().map(str::to_owned);
        self.terminal.update(cx, |terminal, cx| {
            terminal.set_selected_workspace(selected_id, cx);
        });
    }

    fn refresh_session_statuses(&self, cx: &mut Context<Self>) {
        let statuses = terminal_statuses(self.workspace.read(cx).sessions(), |id| {
            self.terminal
                .read(cx)
                .model(id)
                .map(|model| model.read().status.clone())
        });
        self.workspace
            .update(cx, |workspace, cx| workspace.update_statuses(statuses, cx));
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_id = self.workspace.read(cx).selected_id().map(str::to_owned);
        let colors = cx.theme();
        let has_wallpaper = theme::wallpaper(cx).is_some();
        v_flex()
            .p_2()
            .gap_2()
            .size_full()
            .bg(if has_wallpaper {
                colors.background.opacity(0.15)
            } else {
                colors.background
            })
            .child(
                div()
                    .w_full()
                    .h(px(45.))
                    .border_color(colors.border)
                    .bg(if has_wallpaper {
                        Hsla::transparent_black()
                    } else {
                        colors.tab_bar
                    })
                    .child(self.workspace.clone()),
            )
            .child(match selected_id {
                Some(_) => self.terminal.clone().into_any_element(),
                None => render_empty_workspace().into_any_element(),
            })
    }
}
