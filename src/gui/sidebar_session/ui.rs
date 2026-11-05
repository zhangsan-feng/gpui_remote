use super::DraggedSession;
use crate::component::theme;
use crate::domain::session::ConnectionProtocol;
use crate::gui::sidebar_session::{
    CloseSshTunnel, ConnectSession, ConnectSftpSession, DeleteSession, EditSession, OpenSshTunnel,
    SessionComponent,
};
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

impl SessionComponent {
    pub(super) fn render_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let component_colors = cx.theme().colors;
        let ui_colors = theme::CustomerUiTheme::colors(cx);
        let query = self.search_input.read(cx).value().trim().to_lowercase();
        let has_matches = self.sessions.iter().any(|session| {
            format!(
                "{} {} {} {}",
                session.name,
                session.host,
                session.username,
                session.connection_protocol.as_str(),
            )
            .to_lowercase()
            .contains(&query)
        });
        let session_content = if self.sessions.is_empty() {
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
        } else if !has_matches {
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
                        .children(
                            self.sessions
                                .iter()
                                .filter(|session| {
                                    format!(
                                        "{} {} {} {}",
                                        session.name,
                                        session.host,
                                        session.username,
                                        session.connection_protocol.as_str()
                                    )
                                    .to_lowercase()
                                    .contains(&query)
                                })
                                .map(|session| self.session_row(session, cx)),
                        ),
                )
                .child(Scrollbar::vertical(&self.scroll).viewport_from_layout())
                .into_any_element()
        };
        v_flex()
            .on_action(cx.listener(Self::create_active_session))
            .on_action(cx.listener(Self::create_active_sftp_session))
            .on_action(cx.listener(Self::open_ssh_tunnel))
            .on_action(cx.listener(Self::close_ssh_tunnel))
            .on_action(cx.listener(Self::edit_session))
            .on_action(cx.listener(Self::delete_session))
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<DraggedSession>, _, cx| {
                    if !event.bounds.contains(&event.event.position) {
                        this.set_drop_target(None, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.set_drop_target(None, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.set_drop_target(None, cx)),
            )
            .h_full()
            .w_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::sidebar_background(cx))
            .text_color(ui_colors.text_color)
            .child(
                h_flex()
                    .h(px(48.))
                    .px_4()
                    .justify_between()
                    .border_b_1()
                    .border_color(component_colors.sidebar_border)
                    .bg(theme::CustomerUiTheme::panel_background(cx))
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
    }

    fn session_row(
        &self,
        session: &crate::application::session::model::SessionSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = theme::CustomerUiTheme::colors(cx);
        let display_name = if session.name.trim().is_empty() {
            session.host.clone()
        } else {
            session.name.clone()
        };
        let session_id = session.id.clone();
        let target_id = session.id.clone();
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
            .px_3()
            .gap_3()
            .cursor_grab()
            .bg(if self.selected_id.as_ref() == Some(&session.id) {
                colors.select_background
            } else {
                Hsla::transparent_black()
            })
            .hover(move |style| style.bg(colors.hover_background))
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
                            .text_color(colors.text_color)
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
                        let target = (drag.id != hover_id).then(|| {
                            (
                                hover_id.clone(),
                                event.event.position.y > event.bounds.center().y,
                            )
                        });
                        this.set_drop_target(target, cx);
                    }
                }),
            )
            .on_drag(drag, |drag, _, _, cx| cx.new(|_| drag.clone()))
            .on_drop(cx.listener(move |this, drag: &DraggedSession, _, cx| {
                if drag.owner == cx.entity_id() {
                    cx.stop_propagation();
                    this.move_session(&drag.id, &target_id, cx);
                }
            }))
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
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::sidebar_background(cx))
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
                            .text_color(theme::CustomerUiTheme::colors(cx).text_color)
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
