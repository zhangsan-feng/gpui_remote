use super::{
    CloseSshTunnel, ConnectSession, ConnectSftpSession, DeleteSession, DraggedSession, EditSession,
    MoveToGroup, OpenSshTunnel, SessionComponent, tree::SidebarRow,
};
use crate::domain::session::{ConnectionProtocol, SessionGroup};
use crate::gui::component::session_tree::indent;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

impl SessionComponent {
    pub(super) fn render_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let component_colors = cx.theme().colors;
        let ui_colors = cx.theme().colors;
        let rows = self.visible_rows(cx);
        let session_content = if self.sessions.is_empty() && self.groups.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_2()
                .px_4()
                .text_center()
                .child(
                    Icon::new(IconName::SquareTerminal)
                        .large()
                        .text_color(component_colors.muted_foreground),
                )
                .into_any_element()
        } else if rows.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_2()
                .px_4()
                .text_center()
                .child(
                    Icon::new(IconName::Search)
                        .large()
                        .text_color(component_colors.muted_foreground),
                )
                .child(
                    div()
                        .mt_1()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .child("没有匹配的会话"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(component_colors.muted_foreground)
                        .child("试试其他主机名或用户名。"),
                )
                .into_any_element()
        } else {
            div()
                .relative()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .child(
                    v_flex()
                        .id("session-list")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll)
                        .children(rows.into_iter().map(|row| match row {
                            SidebarRow::Group(group, depth, expanded) => {
                                self.group_row(group, depth, expanded, cx)
                            }
                            SidebarRow::Session(session, depth) => {
                                self.session_row(session, depth, cx)
                            }
                        })),
                )
                .child(Scrollbar::vertical(&self.scroll).viewport_from_layout())
                .into_any_element()
        };
        v_flex()
            .relative()
            .on_action(cx.listener(Self::create_active_session))
            .on_action(cx.listener(Self::create_active_sftp_session))
            .on_action(cx.listener(Self::open_ssh_tunnel))
            .on_action(cx.listener(Self::close_ssh_tunnel))
            .on_action(cx.listener(Self::edit_session))
            .on_action(cx.listener(Self::move_to_group))
            .on_action(cx.listener(Self::delete_session))
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<DraggedSession>, _, cx| {
                    if !event.bounds.contains(&event.event.position) {
                        this.set_drop_target(None, cx);
                    } else if event.drag(cx).owner == cx.entity_id() {
                        let from = event.drag(cx).id.clone();
                        if let Some(target) = this.trailing_drop_target(event.event.position, cx) {
                            this.set_drop_target_for(&from, &target, cx);
                        }
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.clear_drag(cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.clear_drag(cx)),
            )
            .h_full()
            .w_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .text_color(ui_colors.foreground)
            .child(
                h_flex()
                    .h(px(48.))
                    .px_4()
                    .justify_between()
                    .border_b_1()
                    .border_color(component_colors.sidebar_border)
                    .bg(cx.theme().group_box)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("会话"),
                    ),
            )
            .child(
                h_flex()
                    .h(px(42.))
                    .px_3()
                    .py_2()
                    .child(Input::new(&self.search_input).small().cleanable(true)),
            )
            .when_some(self.core_err.as_ref(), |this, error| {
                this.child(
                    div()
                        .m_2()
                        .p_2()
                        .rounded_md()
                        .bg(component_colors.danger)
                        .text_xs()
                        .text_color(component_colors.danger_foreground)
                        .child(error.to_string()),
                )
            })
            .child(session_content)
            .child(self.container_drop_surface(cx))
    }

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
                        let rows = this.sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>();
                        log::debug!("session_drag_release_container source={:?} target={:?} active={} position={:?} bounds={bounds:?} inside={} scroll={:?} order={rows:?}", this.drag_source, this.drop_target, cx.has_active_drag(), event.position, bounds.contains(&event.position), this.scroll.offset());
                        log::debug!("session_drag_release_projection scroll_bounds={:?} item_bounds={:?}", this.scroll.bounds(), (0..this.scroll.children_count()).filter_map(|index| this.scroll.bounds_for_item(index)).collect::<Vec<_>>());
                    }
                    owner.update(cx, |this, cx| this.finish_drag_in_blank(event.position, window, cx));
                });
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }

    fn drop_surface(&self, target_id: String, cx: &Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        // Match release against the same visible rectangle used while dragging.
        // Capture runs before child controls can consume the mouse-up event.
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let raw_bounds = bounds;
                let clip = window.content_mask().bounds;
                let bounds = bounds.intersect(&clip);
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                    if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                        let this = owner.read(cx);
                        if this.drag_source.is_some() {
                            log::debug!("session_drag_release_row source={:?} target={} position={:?} raw_bounds={raw_bounds:?} clip={clip:?} visible_bounds={bounds:?} hit={} active={}", this.drag_source, target_id, event.position, bounds.contains(&event.position), cx.has_active_drag());
                        }
                    }
                    if phase == DispatchPhase::Capture
                        && event.button == MouseButton::Left
                        && bounds.contains(&event.position)
                    {
                        owner.update(cx, |this, cx| this.finish_drag_on(&target_id, window, cx));
                    }
                });
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }

    fn group_row(
        &self,
        group: &SessionGroup,
        depth: usize,
        expanded: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = group.id.clone();
        let colors = cx.theme().colors;
        h_flex()
            .id(format!("session-group-{id}"))
            .h(px(34.))
            .w_full()
            .min_w_0()
            .flex_shrink_0()
            .pl(indent(depth))
            .pr_3()
            .gap_2()
            .cursor_pointer()
            .hover(move |style| style.bg(colors.list_hover))
            .child(
                Icon::new(if expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .small(),
            )
            .child(
                Icon::new(if expanded {
                    IconName::FolderOpen
                } else {
                    IconName::Folder
                })
                .small(),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(group.name.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.collapsed_groups.remove(&id) {
                    this.collapsed_groups.insert(id.clone());
                }
                cx.notify();
            }))
            .into_any_element()
    }

    fn session_row(
        &self,
        session: &crate::application::session::model::SessionSummary,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = cx.theme().colors;
        let display_name = if session.name.trim().is_empty() {
            session.host.clone()
        } else {
            session.name.clone()
        };
        let session_id = session.id.clone();
        let drag_owner = cx.entity();
        let hover_id = session.id.clone();
        let drop_side = self
            .drop_target
            .as_ref()
            .filter(|(id, _)| id == &session.id)
            .map(|(_, after)| *after);
        let accent = cx.theme().primary;
        let menu_id = session.id.clone();
        let protocol = session.connection_protocol;
        let configured = session.ssh_tunnel_configured;
        let active = session.ssh_tunnel_active;
        let drag = DraggedSession {
            owner: cx.entity_id(),
            id: session.id.clone(),
            name: display_name.clone(),
            subtitle: format!("{}@{}:{}", session.username, session.host, session.port),
        };
        h_flex()
            .id(format!("session-row-{}", session.id))
            .relative()
            .h(px(58.))
            .w_full()
            .min_w_0()
            .flex_shrink_0()
            .pl(indent(depth))
            .pr_3()
            .gap_3()
            .cursor_grab()
            .bg(if self.selected_id.as_ref() == Some(&session.id) {
                colors.list_active
            } else {
                Hsla::transparent_black()
            })
            .hover(move |style| style.bg(colors.list_hover))
            .child(
                div()
                    .size(px(30.))
                    .flex_shrink_0()
                    .rounded_lg()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
                    .child(IconName::SquareTerminal),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.foreground)
                            .truncate()
                            .child(display_name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .truncate()
                            .child(format!(
                                "{}@{}:{}",
                                session.username, session.host, session.port
                            )),
                    ),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.selected_id = Some(session_id.clone());
                    if event.click_count == 2 {
                        this.create_active_session_by_id(&session_id, cx);
                    }
                    cx.notify();
                }),
            )
            .when_some(drop_side, |row, after| {
                row.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .h(px(3.))
                        .bg(accent)
                        .when(after, |line| line.bottom_0())
                        .when(!after, |line| line.top_0()),
                )
            })
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<DraggedSession>, _, cx| {
                    let drag = event.drag(cx);
                    if drag.owner == cx.entity_id() && event.bounds.contains(&event.event.position)
                    {
                        let from_id = drag.id.clone();
                        this.set_drop_target_for(&from_id, &hover_id, cx);
                    }
                }),
            )
            .on_drag(drag, move |drag, offset, window, cx| {
                log::debug!(
                    "session_drag_start source={} position={:?} offset={offset:?} scale={}",
                    drag.id,
                    window.mouse_position(),
                    window.scale_factor()
                );
                drag_owner.update(cx, |this, _| this.drag_source = Some(drag.id.clone()));
                cx.new(|_| drag.clone())
            })
            .child(self.drop_surface(session.id.clone(), cx))
            .context_menu(move |menu, _, _| {
                let session_id = menu_id.clone();
                let menu = match Some((protocol, configured, active)) {
                    Some((ConnectionProtocol::SshAndSftp, configured, active)) => {
                        let menu = menu
                            .menu_with_icon(
                                "打开 SSH",
                                IconName::SquareTerminal,
                                Box::new(ConnectSession(session_id.clone())),
                            )
                            .menu_with_icon(
                                "打开 SFTP",
                                IconName::FolderOpen,
                                Box::new(ConnectSftpSession(session_id.clone())),
                            );
                        if configured && active {
                            menu.menu_with_icon(
                                "关闭 SSH 隧道",
                                IconName::Settings2,
                                Box::new(CloseSshTunnel(session_id.clone())),
                            )
                        } else if configured {
                            menu.menu_with_icon(
                                "打开 SSH 隧道",
                                IconName::Settings2,
                                Box::new(OpenSshTunnel(session_id.clone())),
                            )
                        } else {
                            menu
                        }
                    }
                    Some((
                        ConnectionProtocol::Mysql
                        | ConnectionProtocol::Pgsql
                        | ConnectionProtocol::Redis,
                        _,
                        _,
                    ))
                    | None => menu,
                };
                menu.menu_with_icon(
                    "编辑",
                    IconName::Settings2,
                    Box::new(EditSession(session_id.clone())),
                )
                .menu_with_icon(
                    "移动到分组",
                    IconName::Folder,
                    Box::new(MoveToGroup(session_id.clone())),
                )
                .menu_with_icon(
                    "删除",
                    IconName::CircleX,
                    Box::new(DeleteSession(session_id)),
                )
            })
            .into_any_element()
    }
}

impl DraggedSession {
    pub(super) fn render_preview(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .h(px(58.))
            .px_3()
            .gap_3()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .shadow_lg()
            .opacity(0.85)
            .child(
                div()
                    .size(px(30.))
                    .rounded_lg()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(cx.theme().sidebar_accent)
                    .text_color(cx.theme().sidebar_accent_foreground)
                    .child(IconName::SquareTerminal),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(cx.theme().colors.foreground)
                            .child(self.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.subtitle.clone()),
                    ),
            )
            .into_any_element()
    }
}
