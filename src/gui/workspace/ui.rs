use gpui::*;
use gpui_component::{ActiveTheme, IconName, v_flex};

use crate::{component::theme, domain::session::Protocol};

use super::Workspace;

impl Workspace {
    pub(super) fn render_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        v_flex()
            // .p_2()
            .gap_2()
            .size_full()
            .bg(theme::CustomerUiTheme::workspace_background(cx))
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .rounded_lg()
            .shadow_lg()
            .child(
                div()
                    .w_full()
                    .h(px(45.))
                    .border_color(colors.border)
                    .bg(theme::CustomerUiTheme::tab_background(cx))
                    .child(self.workspace.clone()),
            )
            .child(match self.active_protocol {
                Some(Protocol::Sftp) => self.sftp.clone().into_any_element(),
                _ => self.terminal.clone().into_any_element(),
            })
    }
}

pub(super) fn render_empty_workspace() -> Div {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(div().text_size(px(36.)).child(IconName::SquareTerminal))
}
