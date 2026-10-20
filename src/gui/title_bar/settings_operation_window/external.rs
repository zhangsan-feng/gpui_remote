use super::SettingsOperationWindow;
use crate::component::window::window_center_options;
use gpui_kit::*;

pub(crate) fn open_settings_window(window: &mut Window, cx: &mut App) {
    open_section(super::SettingsSection::Proxy, window, cx);
}

fn open_section(section: super::SettingsSection, window: &mut Window, cx: &mut App) {
    let mut options = window_center_options(window, size(px(900.), px(640.)));
    options.titlebar = Some(TitlebarOptions {
        title: Some("系统设置".into()),
        appears_transparent: false,
        traffic_light_position: None,
    });
    options.kind = WindowKind::Dialog;
    options.is_resizable = false;
    options.is_minimizable = false;

    let _ = cx.open_window(options, |window, cx| {
        window.on_window_should_close(cx, |window, _| {
            window.remove_window();
            false
        });
        let settings = cx.new(|cx| {
            let mut settings = SettingsOperationWindow::new(window, cx);
            settings.active_section = section;
            settings
        });
        cx.new(|cx| gpui_kit::component::Root::new(settings, window, cx))
    });
}

pub(crate) fn open_network_settings_window(proxy: bool, window: &mut Window, cx: &mut App) {
    open_section(
        if proxy {
            super::SettingsSection::Proxy
        } else {
            super::SettingsSection::SshTunnel
        },
        window,
        cx,
    );
}
