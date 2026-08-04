use crate::component::{draggable_list::DraggableList, theme};
use crate::gui::session::{
    ConnectSession, ConnectSftpSession, DeleteSession, EditSession, SessionComponent,
};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::input::Input;
use gpui_component::menu::PopupMenu;
use gpui_component::*;

impl SessionComponent {
    pub(super) fn render_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let styles = theme::styles(cx);
        v_flex()
            .on_action(cx.listener(Self::create_active_session))
            .on_action(cx.listener(Self::create_active_sftp_session))
            .on_action(cx.listener(Self::edit_session))
            .on_action(cx.listener(Self::delete_session))
            .h_full()
            .w_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(theme::border_color(cx))
            .bg(theme::sidebar_background(cx))
            .text_color(styles.foreground)
            .child(
                h_flex()
                    .h(px(48.))
                    .px_4()
                    .justify_between()
                    .border_b_1()
                    .border_color(colors.sidebar_border)
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
                    .border_b_1()
                    .border_color(colors.sidebar_border)
                    .child(Input::new(&self.search_input).small().cleanable(true)),
            )
            .when_some(self.core_err.as_ref(), |this, error| {
                this.child(
                    div()
                        .m_2()
                        .p_2()
                        .rounded_md()
                        .bg(colors.danger)
                        .text_xs()
                        .text_color(colors.danger_foreground)
                        .child(error.to_string()),
                )
            })
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.draggable_list.clone()),
            )
    }

    pub(super) fn render_item(&mut self, cx: &mut Context<Self>) {
        let styles = theme::styles(cx);
        let mut list = DraggableList::new();
        let session = cx.weak_entity();

        list.set_item_height(px(58.))
            .set_item_bg(Hsla::transparent_black().into())
            .set_item_selected_bg(styles.selected.into())
            .set_item_hover_bg(styles.hover.into())
            .set_context_menu(
                |id: ElementId, menu: PopupMenu, _: &mut Context<PopupMenu>| {
                    let session_id = id.to_string();
                    menu.menu_with_icon(
                        "打开ssh",
                        IconName::SquareTerminal,
                        Box::new(ConnectSession(session_id.clone())),
                    )
                    .menu_with_icon(
                        "打开sftp",
                        IconName::FolderOpen,
                        Box::new(ConnectSftpSession(session_id.clone())),
                    )
                    .menu_with_icon(
                        "编辑",
                        IconName::Settings2,
                        Box::new(EditSession(session_id.clone())),
                    )
                    .menu_with_icon(
                        "删除",
                        IconName::CircleX,
                        Box::new(DeleteSession(session_id)),
                    )
                },
            )
            .on_mouse_down(move |id, event, cx| {
                if event.click_count == 2 {
                    let session_id = id.to_string();
                    let _ = session.update(cx, |session, cx| {
                        session.create_active_session_by_id(&session_id, cx);
                    });
                }
            });
        self.draggable_list = cx.new(|_| list)
    }

    pub(super) fn refer_item(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value().trim().to_lowercase();
        let sessions = self
            .sessions
            .iter()
            .filter(|session| {
                if query.is_empty() {
                    return true;
                }
                format!(
                    "{} {} {} {}",
                    session.name,
                    session.host,
                    session.username,
                    session.protocol.as_str(),
                )
                .to_lowercase()
                .contains(&query)
            })
            .cloned()
            .collect::<Vec<_>>();
        let sidebar_accent = cx.theme().sidebar_accent;
        let sidebar_accent_foreground = cx.theme().sidebar_accent_foreground;
        let sidebar_foreground = theme::styles(cx).foreground;
        let muted_foreground = cx.theme().muted_foreground;

        self.draggable_list.update(cx, move |this, list_cx| {
            this.clear(list_cx);
            for session in sessions {
                this.child(session.id.clone(), move || {
                    let session_id = session.id.clone();
                    h_flex()
                        .id(format!("session-row-{session_id}"))
                        .h(px(58.))
                        .w_full()
                        .px_3()
                        .gap_3()
                        .cursor_grab()
                        .child(
                            div()
                                .size(px(30.))
                                .rounded_lg()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(sidebar_accent)
                                .text_color(sidebar_accent_foreground)
                                .child(IconName::SquareTerminal),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .gap_1()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(sidebar_foreground)
                                        .whitespace_nowrap()
                                        .child(session.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_foreground)
                                        .whitespace_nowrap()
                                        .child(format!(
                                            "{}@{}:{}",
                                            session.username, session.host, session.port
                                        )),
                                ),
                        )
                });
            }
        });
    }
}
