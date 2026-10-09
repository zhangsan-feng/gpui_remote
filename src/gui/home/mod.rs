use gpui_kit::component::{Root, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::sync::{Arc, atomic::AtomicBool};

use crate::{
    component::{resizable_panel::ResizablePanel, theme},
    domain::theme::ThemeSettings,
    global_state::{GlobalState, GlobalStateHandle},
    gui::{sidebar_session::SessionComponent, title_bar::AppTitleBar, workspace::Workspace},
};

pub(crate) fn open_main_window(
    cx: &mut App,
    theme_settings: ThemeSettings,
    theme_updates: tokio::sync::watch::Sender<ThemeSettings>,
    theme_changed: Arc<AtomicBool>,
) {
    let window_options = main_window_options(cx);
    cx.open_window(window_options, move |window, app| {
        window.on_window_should_close(app, |window, _| {
            window.remove_window();
            false
        });
        gpui_kit::init(app);
        theme::init(app, theme_settings, theme_updates, theme_changed);
        window.set_background_appearance(theme::CustomerUiTheme::window_background_appearance(app));

        let global_state = app.new(|_| GlobalState {});
        app.set_global(GlobalStateHandle(global_state));
        let main_window = app.new(|cx| HomeView::new(window, cx));
        app.new(|cx| Root::new(main_window, window, cx))
    })
    .expect("Failed to create app");
}

fn main_window_options(cx: &mut App) -> WindowOptions {
    let mut options = WindowOptions::default();
    let window_size = size(px(1200.), px(700.));
    options.window_bounds = Some(WindowBounds::centered(window_size, cx));
    options.window_min_size = Some(window_size);
    options.titlebar = Some(TitlebarOptions {
        title: None,
        appears_transparent: true,
        traffic_light_position: None,
    });
    options.window_decorations = Some(WindowDecorations::Client);
    options
}

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
                .set_id("home-top_session-resize-handle")
        });
        Self {
            title_bar: cx.new(|cx| AppTitleBar::new(window, cx)),
            content,
        }
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_background_appearance(theme::CustomerUiTheme::window_background_appearance(cx));
        let colors = theme::CustomerUiTheme::colors(cx);
        let wallpaper = colors
            .background_image
            .clone()
            .map(|path| (path, colors.image_opacity));
        div()
            .relative()
            .size_full()
            .bg(colors.background)
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
    }
}
