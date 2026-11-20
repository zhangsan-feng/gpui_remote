use std::sync::{Arc, atomic::AtomicBool};

use gpui_kit::component::Root;
use gpui_kit::*;

use crate::{domain::theme::ThemeSettings, gui::component::theme};

use super::MainWindow;

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
        window.set_background_appearance(WindowBackgroundAppearance::Transparent);

        let main_window = app.new(|cx| MainWindow::new(window, cx));
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
