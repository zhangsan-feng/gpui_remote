use gpui::*;

use super::SettingsOperationWindow;

pub(crate) fn open_settings_window(cx: &mut App) {
    let window_size = size(px(900.), px(640.));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(window_size, cx)),
        window_min_size: Some(window_size),
        titlebar: Some(TitlebarOptions {
            title: Some("系统配置".into()),
            appears_transparent: false,
            traffic_light_position: None,
        }),
        kind: WindowKind::Dialog,
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };

    let _ = cx.open_window(options, |window, cx| {
        let settings = cx.new(|cx| SettingsOperationWindow::new(window, cx));
        cx.new(|cx| gpui_component::Root::new(settings, window, cx))
    });
}
