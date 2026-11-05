mod core;

use gpui_kit::component::{
    ResizableState, Root,
    resizable::{h_resizable, resizable_panel},
    v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::sync::{Arc, atomic::AtomicBool};

use crate::{
    component::theme,
    domain::theme::ThemeSettings,
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
            log::debug!("Main window native close requested: action=remove_window");
            window.remove_window();
            log::debug!("Main window native close handled: action=remove_window");
            false
        });
        gpui_kit::init(app);
        theme::init(app, theme_settings, theme_updates, theme_changed);
        window.set_background_appearance(theme::CustomerUiTheme::window_background_appearance(app));

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
        title: Some(SharedString::from("gpui_remote")),
        appears_transparent: true,
        traffic_light_position: None,
    });
    options.window_decorations = Some(WindowDecorations::Client);
    options
}

pub struct HomeView {
    title_bar: Entity<AppTitleBar>,
    left_session: Entity<SessionComponent>,
    workspace: Entity<Workspace>,
    columns: Entity<ResizableState>,
    diagnostics: core::WindowDiagnostics,
    _bounds_subscription: Subscription,
}

impl HomeView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        core::log_window_bounds(window);
        let scope = cx.entity_id();
        let left_session = cx.new(|cx| SessionComponent::new(scope, window, cx));
        let workspace = cx.new(|cx| Workspace::new(scope, window, cx));
        Self {
            title_bar: cx.new(|cx| AppTitleBar::new(window, cx)),
            left_session,
            workspace,
            columns: cx.new(|_| ResizableState::default()),
            diagnostics: core::WindowDiagnostics::default(),
            _bounds_subscription: cx.observe_window_bounds(window, |_, window, _| {
                core::log_window_bounds(window);
            }),
        }
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_background_appearance(theme::CustomerUiTheme::window_background_appearance(cx));
        let colors = theme::CustomerUiTheme::colors(cx);
        let root_background = theme::CustomerUiTheme::app_background(cx);
        let wallpaper = colors
            .background_image
            .clone()
            .map(|path| (path, colors.image_opacity));
        div()
            .relative()
            .size_full()
            .bg(root_background)
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
                        div().flex_1().min_w_0().min_h_0().overflow_hidden().child(
                            h_resizable("home-columns")
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
                                ),
                        ),
                    ),
            )
    }
}
