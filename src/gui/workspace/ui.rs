use gpui_kit::component::v_flex;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::{component::theme, domain::session::Protocol};

use super::Workspace;

impl Workspace {
    pub(super) fn render_view(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.active_protocol.is_none() {
            return render_empty_workspace(cx).into_any_element();
        }

        let workspace_card = v_flex()
            .gap_2()
            .size_full()
            .bg(theme::CustomerUiTheme::workspace_background(cx))
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .rounded_lg()
            .shadow_sm()
            .child(
                div()
                    .w_full()
                    .h(px(45.))
                    .border_color(theme::CustomerUiTheme::border_color(cx))
                    .bg(super::top_session::workspace_tab_bar_background(cx))
                    .child(self.workspace.clone()),
            )
            .child(match self.active_protocol {
                Some(Protocol::Sftp) => self.sftp.clone().into_any_element(),
                Some(Protocol::Mysql | Protocol::Pgsql | Protocol::Redis) => v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .child("数据库工作区尚未实现")
                    .into_any_element(),
                _ => self.terminal.clone().into_any_element(),
            });

        v_flex()
            .size_full()
            .bg(theme::CustomerUiTheme::app_background(cx))
            .when(
                theme::CustomerUiTheme::uses_soft_default_light_palette(cx),
                |this| this.p_2(),
            )
            .child(workspace_card)
            .into_any_element()
    }
}

pub(super) fn render_empty_workspace(cx: &App) -> Div {
    div()
        .size_full()
        .bg(theme::CustomerUiTheme::app_background(cx))
}
