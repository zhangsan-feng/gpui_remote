use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{v_flex, Root};

use crate::{
    component::{resizable_panel::ResizablePanel, theme},
    gui::{session::SessionComponent, title_bar::AppTitleBar, workspace::Workspace},
};

pub struct HomeView {
    title_bar: Entity<AppTitleBar>,
    content: Entity<ResizablePanel>,
}

impl HomeView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let left_session = cx.new(|cx| SessionComponent::new(window, cx));
        let workspace = cx.new(|cx| Workspace::new(window, cx));
        let content = cx.new(|cx| {
            ResizablePanel::new(left_session, workspace, cx)
                .with_axis(Axis::Horizontal)
                .with_panel_size(252.)
                .with_panel_size_range(190., 480.)
                .set_id("home-session-resize-handle")
        });
        Self {
            title_bar: cx.new(|cx| AppTitleBar::new(window, cx)),
            content,
        }
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_background_appearance(theme::window_background_appearance(cx));
        let styles = theme::styles(cx);
        let wallpaper = styles
            .window_background_img
            .clone()
            .map(|path| (path, theme::wallpaper_opacity(cx)));
        div()
            .relative()
            .size_full()
            .bg(styles.window_background)
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
                    .child(div().flex_1().overflow_hidden().child(self.content.clone())),
            )
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
    }
}
