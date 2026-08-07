use gpui::*;

use super::SettingsOperationWindow;

impl SettingsOperationWindow {
    pub(super) fn reset_font_color(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::component::theme::CustomerUiColor::clear_font_color(cx);
        self.sync_color_pickers(window, cx);
        cx.notify();
    }

    pub(super) fn reset_background_color(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::component::theme::CustomerUiColor::clear_background_color(cx);
        self.sync_color_pickers(window, cx);
        cx.notify();
    }

    pub(super) fn reset_hover_color(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::component::theme::CustomerUiColor::clear_hover_color(cx);
        self.sync_color_pickers(window, cx);
        cx.notify();
    }

    pub(super) fn reset_selected_color(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::component::theme::CustomerUiColor::clear_selected_color(cx);
        self.sync_color_pickers(window, cx);
        cx.notify();
    }

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
                    let result = crate::component::theme::CustomerUiColor::set_wallpaper(&path, cx);
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
        crate::component::theme::CustomerUiColor::clear_wallpaper(cx);
        cx.notify();
    }
}
