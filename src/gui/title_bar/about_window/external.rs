use super::AboutWindow;
use crate::component::window::window_center_options;
use gpui_kit::*;

pub(crate) fn open_about_window(window: &mut Window, cx: &mut App) {
    let mut options = window_center_options(window, size(px(400.), px(220.)));
    options.kind = WindowKind::Normal;
    options.titlebar = Some(TitlebarOptions {
        title: Some("关于".into()),
        appears_transparent: false,
        traffic_light_position: None,
    });
    options.is_resizable = false;
    options.is_minimizable = false;

    if let Err(error) = cx.open_window(options, |window, cx| {
        window.on_window_should_close(cx, |window, _| {
            window.remove_window();
            false
        });
        let about = cx.new(|_| AboutWindow);
        cx.new(|cx| gpui_kit::component::Root::new(about, window, cx))
    }) {
        log::error!("打开关于窗口失败: {error}");
    }
}
