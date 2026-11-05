use gpui_kit::component::{ActiveTheme, h_flex, scroll::Scrollbar, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::{
    component::{color::rgb_to_u32, theme},
    data_context::WorkspaceSummary,
    domain::terminal::TerminalStatus,
};

use super::{DraggedWorkspaceTab, WorkspaceSession};

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
        close_hover_background: component_colors.primary.opacity(0.15),
    }
}

pub fn workspace_tab_bar_background(cx: &App) -> Hsla {
    workspace_tab_palette(cx).background
}

impl WorkspaceSession {
    pub(super) fn render_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .size_full()
            .min_w_0()
            .overflow_hidden()
            .child(
                div()
                    .id("workspace-tabs-scrollbar")
                    .relative()
                    .w_full()
                    .h(px(5.))
                    .flex_shrink_0()
                    .child(Scrollbar::horizontal(&self.tabs_scroll).viewport_from_layout()),
            )
            .child(
                h_flex()
                    .id("workspace-tabs")
                    .w_full()
                    .h(px(40.))
                    .min_w_0()
                    .flex_shrink_0()
                    .overflow_x_scroll()
                    .track_scroll(&self.tabs_scroll)
                    .on_drag_move(cx.listener(
                        |this, event: &DragMoveEvent<DraggedWorkspaceTab>, _, cx| {
                            if !event.bounds.contains(&event.event.position) {
                                this.set_drop_target(None, cx);
                            }
                        },
                    ))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.set_drop_target(None, cx)),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.set_drop_target(None, cx)),
                    )
                    .children(self.sessions.iter().map(|session| {
                        workspace_tab(
                            session.id.clone(),
                            session.profile.clone(),
                            session.profile.status.clone(),
                            self.selected_id.as_deref() == Some(session.id.as_str()),
                            self.drop_target
                                .as_ref()
                                .filter(|(id, _)| id == &session.id)
                                .map(|(_, after)| *after),
                            cx.entity(),
                            cx,
                        )
                    })),
            )
            .into_any_element()
    }
}

pub(in crate::gui::workspace) fn workspace_tab(
    workspace_id: String,
    profile: WorkspaceSummary,
    status: TerminalStatus,
    selected: bool,
    drop_side: Option<bool>,
    workspace: Entity<WorkspaceSession>,
    cx: &App,
) -> AnyElement {
    let palette = workspace_tab_palette(cx);
    let target_id = workspace_id.clone();
    let hover_id = workspace_id.clone();
    let close_id = workspace_id.clone();
    let drag = DraggedWorkspaceTab {
        owner: workspace.entity_id(),
        workspace_id: workspace_id.clone(),
        profile: profile.clone(),
        selected,
    };
    tab_surface(&profile, &status, selected, cx)
        .id(format!("workspace-tab-{workspace_id}"))
        .cursor_grab()
        .hover(move |style| {
            style.bg(if selected {
                palette.selected_background
            } else {
                palette.hover_background
            })
        })
        .child(
            close_icon(cx)
                .hover(move |style| {
                    style
                        .bg(palette.close_hover_background)
                        .text_color(palette.accent)
                })
                .id(format!("close-workspace-{close_id}"))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click({
                    let workspace = workspace.clone();
                    move |_, _, cx| {
                        cx.stop_propagation();
                        workspace.update(cx, |this, cx| this.close(&close_id, cx));
                    }
                }),
        )
        .when_some(drop_side, |tab, after| {
            tab.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .w(px(3.))
                    .bg(palette.accent)
                    .when(after, |line| line.right_0())
                    .when(!after, |line| line.left_0()),
            )
        })
        .on_drag_move({
            let workspace = workspace.clone();
            move |event: &DragMoveEvent<DraggedWorkspaceTab>, _, cx| {
                let drag = event.drag(cx);
                if drag.owner == workspace.entity_id()
                    && event.bounds.contains(&event.event.position)
                {
                    let target = (drag.workspace_id != hover_id).then(|| {
                        (
                            hover_id.clone(),
                            event.event.position.x > event.bounds.center().x,
                        )
                    });
                    workspace.update(cx, |this, cx| this.set_drop_target(target, cx));
                }
            }
        })
        .on_drag(drag, |drag, _, _, cx| {
            log::info!(
                "Workspace tab drag started: workspace_id={}, selected={}",
                drag.workspace_id,
                drag.selected
            );
            cx.new(|_| drag.clone())
        })
        .on_drop({
            let workspace = workspace.clone();
            move |drag: &DraggedWorkspaceTab, window, cx| {
                if drag.owner == workspace.entity_id() {
                    cx.stop_propagation();
                    workspace.update(cx, |this, cx| {
                        this.move_tab(&drag.workspace_id, &target_id, window, cx)
                    });
                }
            }
        })
        .on_click(move |_, _, cx| {
            workspace.update(cx, |this, cx| this.activate(&workspace_id, cx));
        })
        .into_any_element()
}

fn tab_surface(
    profile: &WorkspaceSummary,
    status: &TerminalStatus,
    selected: bool,
    cx: &App,
) -> Div {
    let palette = workspace_tab_palette(cx);
    let title = if profile.title.trim().is_empty() {
        profile.host.clone()
    } else {
        profile.title.clone()
    };
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
        .relative()
        .w_auto()
        .flex_shrink_0()
        .h(px(40.))
        .p_2()
        .gap_2()
        .rounded_lg()
        .border_1()
        .border_color(if selected {
            palette.selected_border
        } else {
            palette.border
        })
        .text_color(palette.foreground)
        .bg(if selected {
            palette.selected_background
        } else {
            palette.background
        })
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
                .child(title),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(status_color)
                .child(status_text),
        )
}

fn close_icon(cx: &App) -> Div {
    let palette = workspace_tab_palette(cx);
    div()
        .size(px(22.))
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .text_base()
        .font_weight(FontWeight::MEDIUM)
        .text_color(palette.muted_foreground)
        .child("×")
}

impl DraggedWorkspaceTab {
    pub(super) fn render_preview(&self, cx: &mut Context<Self>) -> AnyElement {
        tab_surface(&self.profile, &self.profile.status, self.selected, cx)
            .child(close_icon(cx))
            .opacity(0.85)
            .shadow_lg()
            .into_any_element()
    }
}
