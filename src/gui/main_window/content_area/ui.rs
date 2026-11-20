use gpui_kit::component::ActiveTheme;
use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::{domain::session::Protocol, gui::component::theme};

use super::Workspace;

impl Workspace {
    pub(super) fn render_view(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.active_protocol.is_none() {
            return render_empty_workspace(cx).into_any_element();
        }

        let workspace_card = v_flex()
            .gap_2()
            .size_full()
            .min_w_0()
            .bg(workspace_background(cx))
            .border_1()
            .border_color(cx.theme().border)
            .rounded_lg()
            .shadow_sm()
            .child(
                div()
                    .w_full()
                    .h(px(45.))
                    .min_w_0()
                    .flex_shrink_0()
                    .overflow_hidden()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().tab_bar)
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
            .p_2()
            .child(workspace_card)
            .into_any_element()
    }
}

pub(super) fn render_empty_workspace(cx: &App) -> Div {
    div().size_full().bg(workspace_background(cx))
}

pub(super) fn workspace_background(cx: &App) -> Hsla {
    if theme::CustomerUiColor::has_wallpaper(cx) {
        cx.theme().transparent
    } else {
        cx.theme().background
    }
}
