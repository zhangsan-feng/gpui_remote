use crate::component::color::rgb_to_u32;
use crate::component::draggable_list::DraggableList;
use crate::gui::session::{ConnectSession, DeleteSession, EditSession, SessionComponent};
use gpui::*;
use gpui_component::menu::PopupMenu;
use gpui_component::*;

impl SessionComponent {
    pub(super) fn render_item(&mut self, cx: &mut Context<Self>) {
        let mut list = DraggableList::new();
        let session = cx.weak_entity();

        list.set_item_height(px(58.))
            .set_item_bg(rgb_to_u32(249, 247, 251))
            .set_item_selected_bg(rgb_to_u32(238, 229, 246))
            .set_item_hover_bg(rgb_to_u32(242, 237, 247))
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
                                .bg(rgb_to_u32(235, 230, 239))
                                .text_color(rgb_to_u32(93, 61, 116))
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
                                        .text_color(rgb_to_u32(55, 47, 67))
                                        .whitespace_nowrap()
                                        .child(session.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb_to_u32(132, 123, 143))
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
