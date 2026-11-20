use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{
    resizable::{h_resizable, resizable_panel},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::gui::component::theme;

use super::{MainWindow, bottom_sidebar, right_sidebar};

const SHOW_RIGHT_SIDEBAR: bool = false;

impl MainWindow {
    pub(super) fn render_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        let wallpaper = theme::CustomerUiColor::wallpaper(cx).filter(|(_, opacity)| *opacity > 0.);
        let columns = h_resizable("home-columns")
            .with_state(&self.columns)
            .child(
                resizable_panel()
                    .size(px(252.))
                    .size_range(px(190.)..px(480.))
                    .fixed()
                    .flex_none()
                    .child(self.left_session.clone()),
            )
            .child(
                resizable_panel()
                    .size_range(px(360.)..Pixels::MAX)
                    .child(self.workspace.clone()),
            );

        div()
            .relative()
            .size_full()
            .bg(colors.background)
            .child(self.diagnostics.layout_probe())
            .when_some(wallpaper, |this, (path, opacity)| {
                this.child(
                    img(path)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .opacity(opacity),
                )
            })
            .child(
                v_flex()
                    .relative()
                    .size_full()
                    .child(self.title_bar.clone())
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex_1()
                                    .h_full()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .child(columns),
                            )
                            .when(SHOW_RIGHT_SIDEBAR, |body| {
                                body.child(right_sidebar::render(&colors))
                            }),
                    )
                    .child(bottom_sidebar::render(&colors)),
            )
    }
}
