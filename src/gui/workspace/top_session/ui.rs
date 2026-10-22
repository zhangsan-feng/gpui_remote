use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::*;

use crate::{
    component::{color::rgb_to_u32, draggable_list::DraggableList, theme},
    data_context::WorkspaceSummary,
    domain::terminal::TerminalStatus,
};

use super::WorkspaceSession;

#[derive(Clone, Copy)]
struct WorkspaceTabPalette {
    background: Hsla,
    selected_background: Hsla,
    hover_background: Hsla,
    border: Hsla,
    selected_border: Hsla,
    accent: Hsla,
    accent_soft: Hsla,
    accent_foreground: Hsla,
    foreground: Hsla,
    muted_foreground: Hsla,
    close_hover_background: Hsla,
}

fn workspace_tab_palette(cx: &App) -> WorkspaceTabPalette {
    let app_colors = theme::CustomerUiTheme::colors(cx);
    let component_colors = cx.theme().colors;
    WorkspaceTabPalette {
        background: theme::CustomerUiTheme::tab_background(cx),
        selected_background: app_colors.workspace_select_color,
        hover_background: app_colors.hover_background,
        border: app_colors.border_color,
        selected_border: component_colors.list_active_border,
        accent: component_colors.primary,
        accent_soft: component_colors.accent,
        accent_foreground: component_colors.accent_foreground,
        foreground: app_colors.workspace_text_color,
        muted_foreground: component_colors.muted_foreground,
        close_hover_background: app_colors.hover_background,
    }
}

pub fn workspace_tab_bar_background(cx: &App) -> Hsla {
    workspace_tab_palette(cx).background
}

pub(super) fn new_workspace_tabs(_: &App) -> DraggableList {
    let mut tabs = DraggableList::new();
    tabs.set_axis(Axis::Horizontal)
        .set_item_height(px(40.))
        .set_item_bg_provider(|cx| workspace_tab_palette(cx).background.into())
        .set_item_selected_bg_provider(|cx| workspace_tab_palette(cx).selected_background.into())
        .set_item_hover_bg_provider(|cx| workspace_tab_palette(cx).hover_background.into());
    tabs
}

pub(in crate::gui::workspace) fn workspace_tab(
    workspace_id: String,
    profile: WorkspaceSummary,
    status: TerminalStatus,
    selected: bool,
    workspace: Entity<WorkspaceSession>,
    cx: &App,
) -> AnyElement {
    let palette = workspace_tab_palette(cx);
    let session_id = workspace_id.clone();
    let close_id = workspace_id;
    let protocol = if profile.is_ssh_tunnel {
        "SSH 隧道"
    } else {
        profile.protocol.as_str()
    };
    let (status_color, status_text) = match status {
        TerminalStatus::Connecting => (rgb_to_u32(217, 119, 6), "连接中"),
        TerminalStatus::Connected => (rgb_to_u32(22, 163, 74), "已连接"),
        TerminalStatus::Disconnected => (rgb_to_u32(107, 114, 128), "已断开"),
        TerminalStatus::Failed => (rgb_to_u32(220, 38, 38), "失败"),
    };
    let (badge_background, badge_foreground) =
        if profile.protocol == crate::domain::session::Protocol::Sftp {
            (
                cx.theme().sidebar_accent,
                cx.theme().sidebar_accent_foreground,
            )
        } else {
            (palette.accent_soft, palette.accent_foreground)
        };
    h_flex()
        .id(format!("workspace-tab-{session_id}"))
        .w_auto()
        .h_full()
        .p_2()
        .gap_2()
        .cursor_pointer()
        .rounded_lg()
        .border_1()
        .border_color(if selected {
            palette.selected_border
        } else {
            palette.border
        })
        .text_color(palette.foreground)
        .child(
            div()
                .h(px(20.))
                .px_2()
                .flex()
                .items_center()
                .flex_shrink_0()
                .rounded_md()
                .bg(badge_background)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(badge_foreground)
                .child(protocol),
        )
        .child(
            div()
                .flex_shrink_0()
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
                .id(format!("close-workspace-top_session-{close_id}"))
                .size(px(22.))
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .text_base()
                .font_weight(FontWeight::MEDIUM)
                .text_color(palette.muted_foreground)
                .hover(move |style| {
                    style
                        .bg(palette.close_hover_background)
                        .text_color(palette.accent)
                })
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
        .into_any_element()
}
