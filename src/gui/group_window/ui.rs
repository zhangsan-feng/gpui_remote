use gpui_kit::{
    component::{
        ActiveTheme, Disableable, Icon, IconName, Sizable,
        button::{Button, ButtonVariants},
        h_flex,
        input::Input,
        v_flex,
    },
    prelude::FluentBuilder,
    *,
};

use crate::gui::component::session_tree::{group_rows, indent};

use super::MoveSessionWindow;

impl MoveSessionWindow {
    pub(super) fn render_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let session_name = if self.session.name.trim().is_empty() {
            &self.session.host
        } else {
            &self.session.name
        };
        let tree = v_flex()
            .id("move-session-group-tree")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .border_1()
            .border_color(colors.border)
            .rounded_md()
            .bg(colors.sidebar)
            .child(self.destination_row(None, "根目录（未分组）", 0, cx))
            .children(group_rows(&self.groups).into_iter().map(|(group, depth)| {
                self.destination_row(Some(&group.id), &group.name, depth + 1, cx)
            }));

        let has_name = !self.name.read(cx).value().trim().is_empty();
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(
                div()
                    .text_sm()
                    .truncate()
                    .child(format!("会话：{session_name}")),
            )
            .child(tree)
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_sm().child("在所选位置新建分组"))
                    .child(Input::new(&self.name)),
            )
            .when_some(self.error.clone(), |this, error| {
                this.child(div().text_sm().text_color(colors.danger).child(error))
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-move-session")
                            .ghost()
                            .label("取消")
                            .disabled(self.busy)
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("move-session-existing")
                            .outline()
                            .label("移动")
                            .disabled(self.busy || self.selected == self.session.group_id)
                            .on_click(cx.listener(Self::move_existing)),
                    )
                    .child(
                        Button::new("move-session-create")
                            .primary()
                            .label("新建并移动")
                            .disabled(self.busy || !has_name)
                            .on_click(cx.listener(Self::create_and_move)),
                    ),
            )
    }

    fn destination_row(
        &self,
        id: Option<&str>,
        name: &str,
        depth: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme().colors;
        let id = id.map(str::to_owned);
        let selected = self.selected == id;
        Button::new(format!("group-target-{}", id.as_deref().unwrap_or("root")))
            .ghost()
            .disabled(self.busy)
            .w_full()
            .justify_start()
            .pl(indent(depth))
            .when(selected, |button| button.bg(colors.list_active))
            .child(Icon::new(IconName::Folder).small())
            .child(div().min_w_0().truncate().child(name.to_owned()))
            .when(selected, |button| {
                button.child(Icon::new(IconName::Check).small())
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected = id.clone();
                this.error = None;
                cx.notify();
            }))
    }
}
