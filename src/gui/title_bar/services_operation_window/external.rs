use super::ServicesOperationWindow;
use crate::component::window::window_center_options;
use gpui_kit::*;

pub(crate) fn open_services_window(window: &mut Window, cx: &mut App) {
    let mut options = window_center_options(window, size(px(900.), px(640.)));
    options.titlebar = Some(TitlebarOptions {
        title: None,
        appears_transparent: true,
        traffic_light_position: None,
    });
    options.window_decorations = Some(WindowDecorations::Client);
    options.kind = WindowKind::Dialog;
    options.is_resizable = false;
    options.is_minimizable = false;

    match cx.open_window(options, |window, cx| {
        window.on_window_should_close(cx, |window, _| {
            log::debug!("services_window_close_requested");
            window.remove_window();
            false
        });
        let services = cx.new(|cx| ServicesOperationWindow::new(window, cx));
        cx.new(|cx| gpui_kit::component::Root::new(services, window, cx))
    }) {
        Ok(_) => log::info!("services_window_opened"),
        Err(error) => log::error!("services_window_open_failed: {error}"),
    }
}
