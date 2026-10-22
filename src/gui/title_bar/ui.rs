use crate::component::theme;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::AppTitleBar;

#[derive(Clone, Copy)]
pub(super) struct TitleBarPalette {
    pub(super) background: Hsla,
    pub(super) border: Hsla,
    pub(super) foreground: Hsla,
    pub(super) control_hover: Hsla,
    pub(super) close_hover: Hsla,
}

pub(super) fn title_bar_palette(cx: &App) -> TitleBarPalette {
    let app_colors = theme::CustomerUiTheme::colors(cx);
    if theme::CustomerUiTheme::uses_soft_default_light_palette(cx) {
        TitleBarPalette {
            background: app_colors.title_bar_background,
            border: app_colors.border_color,
            foreground: app_colors.text_color,
            control_hover: app_colors.hover_background,
            close_hover: rgb(0xfee2e2).into(),
        }
    } else {
        TitleBarPalette {
            background: theme::CustomerUiTheme::title_background(cx),
            border: theme::CustomerUiTheme::border_color(cx),
            foreground: app_colors.text_color,
            control_hover: app_colors.hover_background,
            close_hover: app_colors.hover_background,
        }
    }
}

impl AppTitleBar {
    pub(super) fn window_button(
        &self,
        id: &'static str,
        label: &'static str,
        control: WindowControlArea,
        foreground: Hsla,
        hover_color: Hsla,
        cx: &Context<Self>,
    ) -> AnyElement {
        div()
            .id(id)
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(foreground)
            .hover(move |style| style.bg(hover_color))
            .window_control_area(control)
            .when(cfg!(target_os = "linux"), move |this| {
                this.on_click(cx.listener(move |_, _, window, _| match control {
                    WindowControlArea::Min => window.minimize_window(),
                    WindowControlArea::Max => window.zoom_window(),
                    _ => {}
                }))
            })
            .child(label)
            .into_any_element()
    }

    pub(super) fn close_button(
        &self,
        foreground: Hsla,
        hover_color: Hsla,
        cx: &Context<Self>,
    ) -> AnyElement {
        div()
            .id("window-close")
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(foreground)
            .hover(move |style| style.bg(hover_color))
            .on_click(cx.listener(|_, _, window, _| {
                log::debug!("Main window custom close clicked: action=remove_window");
                window.remove_window();
                log::debug!("Main window custom close handled: action=remove_window");
            }))
            .child("×")
            .into_any_element()
    }
}
