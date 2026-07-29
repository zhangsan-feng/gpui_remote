use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    slider::Slider,
    v_flex,
};

use crate::component::theme;

use super::SettingsOperationWindow;

impl SettingsOperationWindow {
    pub(super) fn wallpaper_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let wallpaper = theme::wallpaper(cx);
        let wallpaper_name = wallpaper
            .as_ref()
            .and_then(|(path, _)| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("未选择图片")
            .to_owned();

        v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("背景图片"),
            )
            .child(
                v_flex()
                    .p_4()
                    .gap_3()
                    .rounded_xl()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.title_bar)
                    .child(
                        h_flex()
                            .gap_3()
                            .justify_between()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .child(Icon::new(IconName::File).small())
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(wallpaper_name),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(colors.muted_foreground)
                                            .child("PNG、JPG、JPEG 或 WebP，按 Cover 等比覆盖"),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("choose-wallpaper")
                                            .outline()
                                            .small()
                                            .label(if wallpaper.is_some() {
                                                "更换图片"
                                            } else {
                                                "选择图片"
                                            })
                                            .on_click(cx.listener(Self::choose_wallpaper)),
                                    )
                                    .when(wallpaper.is_some(), |this| {
                                        this.child(
                                            Button::new("clear-wallpaper")
                                                .ghost()
                                                .small()
                                                .label("清除")
                                                .on_click(cx.listener(Self::clear_wallpaper)),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .text_xs()
                                    .text_color(colors.muted_foreground)
                                    .child("图片透明度")
                                    .child(format!("{:.0}%", theme::wallpaper_opacity(cx) * 100.)),
                            )
                            .child(Slider::new(&self.wallpaper_opacity).horizontal().w_full()),
                    )
                    .when_some(self.wallpaper_error.clone(), |this, error| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.danger_foreground)
                                .child(error),
                        )
                    }),
            )
    }
}
