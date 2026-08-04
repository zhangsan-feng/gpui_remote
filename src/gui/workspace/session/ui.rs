use gpui::*;
use gpui_component::{h_flex, ThemeColor};

use crate::{
    component::{color::rgb_to_u32, draggable_list::DraggableList, theme},
    domain::{session::SessionProfile, terminal::TerminalStatus},
};

use super::WorkspaceSession;

pub(super) fn new_workspace_tabs(cx: &App) -> DraggableList {
    let styles = theme::styles(cx);
    let item_bg = theme::tab_background(cx).into();
    let selected_bg = styles.selected.into();
    let mut tabs = DraggableList::new();
    tabs.set_axis(Axis::Horizontal)
        .set_item_width(px(240.))
        .set_item_height(px(34.))
        .set_item_bg(item_bg)
        .set_item_selected_bg(selected_bg)
        .set_item_hover_bg(styles.hover.into());
    tabs
}

pub(in crate::gui::workspace) fn workspace_tab(
    workspace_id: String,
    profile: SessionProfile,
    status: TerminalStatus,
    workspace: Entity<WorkspaceSession>,
    colors: ThemeColor,
) -> impl IntoElement {
    let session_id = workspace_id.clone();
    let close_id = workspace_id;
    let protocol = profile.protocol.as_str();
    let (status_color, status_text) = match status {
        TerminalStatus::Connecting => (rgb_to_u32(217, 119, 6), "连接中"),
        TerminalStatus::Connected => (rgb_to_u32(22, 163, 74), "已连接"),
        TerminalStatus::Disconnected => (rgb_to_u32(107, 114, 128), "已断开"),
        TerminalStatus::Failed => (rgb_to_u32(220, 38, 38), "失败"),
    };
    h_flex()
        .id(format!("workspace-tab-{session_id}"))
        .size_full()
        .p_2()
        .gap_2()
        .cursor_pointer()
        .rounded_lg()
        .border_1()
        .border_color(colors.border)
        .text_color(colors.foreground)
        .child(
            div()
                .h(px(20.))
                .px_2()
                .flex()
                .items_center()
                .flex_shrink_0()
                .rounded_md()
                .bg(colors.accent)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.accent_foreground)
                .child(protocol),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .whitespace_nowrap()
                .overflow_hidden()
                .child(format!("{}", profile.host)),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(status_color)
                .child(status_text),
        )
        .child(
            div()
                .id(format!("close-workspace-session-{close_id}"))
                .size(px(22.))
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .text_base()
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.muted_foreground)
                .hover(|style| style.bg(colors.accent).text_color(colors.accent_foreground))
                .child("×")
                .on_click({
                    let workspace = workspace.clone();
                    move |_, _, cx| {
                        cx.stop_propagation();
                        workspace.update(cx, |workspace, cx| workspace.close(&close_id, cx));
                    }
                }),
        )
        .on_click(move |_, _, cx| {
            workspace.update(cx, |workspace, cx| workspace.activate(&session_id, cx));
        })
}
