use gpui_kit::component::{ActiveTheme, h_flex, scroll::Scrollbar, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::{data_context::WorkspaceSummary, domain::terminal::TerminalStatus};

use super::{DraggedWorkspaceTab, WorkspaceSession};

impl WorkspaceSession {
    fn container_drop_surface(&self, cx: &Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                    if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                        return;
                    }
                    let this = owner.read(cx);
                    if this.drag_source.is_some() {
                        let order = this.sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>();
                        log::debug!("tab_drag_release_container source={:?} target={:?} active={} position={:?} bounds={bounds:?} inside={} scroll={:?} order={order:?}", this.drag_source, this.drop_target, cx.has_active_drag(), event.position, bounds.contains(&event.position), this.tabs_scroll.offset());
                    }
                    owner.update(cx, |this, cx| this.finish_drag_in_blank(event.position, window, cx));
                });
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }

    pub(super) fn render_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .relative()
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
                            } else if event.drag(cx).owner == cx.entity_id() {
                                let from = event.drag(cx).workspace_id.clone();
                                if let Some(target) =
                                    this.trailing_drop_target(event.event.position)
                                {
                                    this.set_drop_target_for(&from, &target, cx);
                                }
                            }
                        },
                    ))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.clear_drag(cx)),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.clear_drag(cx)),
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
            .child(self.container_drop_surface(cx))
            .into_any_element()
    }
}

pub(in crate::gui::main_window::content_area) fn workspace_tab(
    workspace_id: String,
    profile: WorkspaceSummary,
    status: TerminalStatus,
    selected: bool,
    drop_side: Option<bool>,
    workspace: Entity<WorkspaceSession>,
    cx: &App,
) -> AnyElement {
    let colors = cx.theme().colors;
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
                colors.tab_active
            } else {
                colors.list_hover
            })
        })
        .child(
            close_icon(cx)
                .hover(move |style| {
                    style
                        .bg(colors.primary.opacity(0.15))
                        .text_color(colors.primary)
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
                    .bg(colors.primary)
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
                    let from_id = drag.workspace_id.clone();
                    workspace.update(cx, |this, cx| {
                        this.set_drop_target_for(&from_id, &hover_id, cx)
                    });
                }
            }
        })
        .on_drag(drag, {
            let workspace = workspace.clone();
            move |drag, offset, window, cx| {
                log::debug!(
                    "tab_drag_start source={} position={:?} offset={offset:?} scale={}",
                    drag.workspace_id,
                    window.mouse_position(),
                    window.scale_factor()
                );
                workspace.update(cx, |this, _| {
                    this.drag_source = Some(drag.workspace_id.clone())
                });
                log::debug!(
                    "Workspace tab drag started: workspace_id={}, selected={}",
                    drag.workspace_id,
                    drag.selected
                );
                cx.new(|_| drag.clone())
            }
        })
        .child(tab_drop_surface(target_id, workspace.clone()))
        .on_click(move |_, _, cx| {
            workspace.update(cx, |this, cx| this.activate(&workspace_id, cx));
        })
        .into_any_element()
}

fn tab_drop_surface(target_id: String, workspace: Entity<WorkspaceSession>) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let raw_bounds = bounds;
            let clip = window.content_mask().bounds;
            let bounds = bounds.intersect(&clip);
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                    let this = workspace.read(cx);
                    if this.drag_source.is_some() {
                        log::debug!("tab_drag_release_row source={:?} target={} position={:?} raw_bounds={raw_bounds:?} clip={clip:?} visible_bounds={bounds:?} hit={} active={}", this.drag_source, target_id, event.position, bounds.contains(&event.position), cx.has_active_drag());
                    }
                }
                if phase == DispatchPhase::Capture
                    && event.button == MouseButton::Left
                    && bounds.contains(&event.position)
                {
                    workspace.update(cx, |this, cx| this.finish_drag_on(&target_id, window, cx));
                }
            });
        },
    )
    .absolute()
    .inset_0()
    .size_full()
}

fn tab_surface(
    profile: &WorkspaceSummary,
    status: &TerminalStatus,
    selected: bool,
    cx: &App,
) -> Div {
    let colors = cx.theme().colors;
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
        TerminalStatus::Connecting => (colors.warning, "连接中"),
        TerminalStatus::Connected => (colors.success, "已连接"),
        TerminalStatus::Disconnected => (colors.muted_foreground, "已断开"),
        TerminalStatus::Failed => (colors.danger, "失败"),
    };
    let (badge_background, badge_foreground) =
        if profile.protocol == crate::domain::session::Protocol::Sftp {
            (
                cx.theme().sidebar_accent,
                cx.theme().sidebar_accent_foreground,
            )
        } else {
            (colors.accent, colors.accent_foreground)
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
            colors.list_active_border
        } else {
            colors.border
        })
        .text_color(if selected {
            colors.tab_active_foreground
        } else {
            colors.tab_foreground
        })
        .bg(if selected {
            colors.tab_active
        } else {
            colors.tab
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
    let colors = cx.theme().colors;
    div()
        .size(px(22.))
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .text_base()
        .font_weight(FontWeight::MEDIUM)
        .text_color(colors.muted_foreground)
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
