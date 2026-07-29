use crate::component::{draggable_list::DraggableList, theme};
use crate::gui::session::{ConnectSession, DeleteSession, EditSession, SessionComponent};
use gpui::*;
use gpui_component::menu::PopupMenu;
use gpui_component::*;

impl SessionComponent {
    pub(super) fn render_item(&mut self, cx: &mut Context<Self>) {
        let colors = cx.theme();
        let mut list = DraggableList::new();
        let session = cx.weak_entity();

        list.set_item_height(px(58.))
            .set_item_bg(Hsla::transparent_black().into())
            .set_item_selected_bg(colors.sidebar_accent.into())
            .set_item_hover_bg(colors.list_hover.into())
            .set_context_menu(
                |id: ElementId, menu: PopupMenu, _: &mut Context<PopupMenu>| {
                    let session_id = id.to_string();
                    menu.menu_with_icon(
                        "连接",
                        IconName::SquareTerminal,
                        Box::new(ConnectSession(session_id.clone())),
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
        let sessions = self.sessions.clone();
        let sidebar_accent = cx.theme().sidebar_accent;
        let sidebar_accent_foreground = cx.theme().sidebar_accent_foreground;
        let sidebar_foreground = super::contrast_text(theme::sidebar_base_color(cx));
        let muted_foreground = cx.theme().muted_foreground;

        self.draggable_list.update(cx, move |this, _cx| {
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
