use gpui::*;

use super::SettingsOperationWindow;

impl SettingsOperationWindow {
    pub(super) fn choose_wallpaper(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.wallpaper_error = None;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("选择背景图片".into()),
        });
        let this = cx.weak_entity();
        window
            .spawn(cx, async move |cx| {
                let paths = match receiver.await {
                    Ok(Ok(Some(paths))) => paths,
                    _ => return Ok::<(), anyhow::Error>(()),
                };
                let Some(path) = paths.into_iter().next() else {
                    return Ok(());
                };
                cx.update(|_, cx| {
                    let result = crate::component::theme::set_wallpaper(&path, cx);
                    let _ = this.update(cx, |this, cx| {
                        this.wallpaper_error = result.err();
                        cx.notify();
                    });
                })?;
                Ok(())
            })
            .detach();
    }

    pub(super) fn clear_wallpaper(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.wallpaper_error = None;
        crate::component::theme::clear_wallpaper(cx);
        cx.notify();
    }
}

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
