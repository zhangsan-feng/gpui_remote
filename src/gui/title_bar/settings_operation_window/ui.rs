use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, color_picker::ColorPicker, h_flex, v_flex,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::color_picker::ColorPickerState;
use gpui_component::slider::{Slider, SliderState};
use crate::component::theme::{self, AppTheme};

use super::SettingsOperationWindow;

impl SettingsOperationWindow {
    pub(super) fn custom_color_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let selected = theme::active(cx) == AppTheme::Custom;
        let featured_colors = AppTheme::ALL
            .into_iter()
            .map(|theme| theme.preset().accent.into())
            .collect();

        h_flex()
            .p_4()
            .gap_4()
            .justify_between()
            .rounded_xl()
            .border_1()
            .border_color(if selected {
                colors.primary
            } else {
                colors.border
            })
            .bg(colors.title_bar)
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Icon::new(IconName::Palette).small())
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("自定义主题"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child("选择一个主色，自动生成背景、侧栏、边框与交互颜色。"),
                    ),
            )
            .child(
                ColorPicker::new(&self.color_picker)
                    .label("主色")
                    .featured_colors(featured_colors),
            )
    }

    pub(super) fn sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        v_flex()
            .w(px(184.))
            .flex_shrink_0()
            .p_3()
            .gap_2()
            .border_r_1()
            .border_color(colors.sidebar_border)
            .bg(colors.sidebar)
            .child(
                div()
                    .px_3()
                    .pt_2()
                    .pb_3()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("系统配置"),
            )
            .child(
                h_flex()
                    .h(px(38.))
                    .px_3()
                    .gap_2()
                    .rounded_lg()
                    .bg(colors.sidebar_accent)
                    .text_color(colors.sidebar_accent_foreground)
                    .child(Icon::new(IconName::Palette).small())
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child("外观"),
                    ),
            )
    }

    pub(super) fn theme_card(
        &self,
        selected_theme: AppTheme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme();
        let preset = selected_theme.preset();
        let selected = theme::active(cx) == selected_theme;

        v_flex()
            .id(format!("theme-{:?}", preset.id))
            .relative()
            .h(px(138.))
            .p_4()
            .gap_3()
            .rounded_xl()
            .border_1()
            .border_color(if selected {
                colors.primary
            } else {
                colors.border
            })
            .bg(if selected {
                colors.accent
            } else {
                colors.title_bar
            })
            .hover(|style| style.bg(colors.list_hover))
            .cursor_pointer()
            .when(selected, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_3()
                        .right_3()
                        .size(px(24.))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(colors.primary)
                        .text_color(colors.primary_foreground)
                        .child(Icon::new(IconName::Check).xsmall()),
                )
            })
            .child(h_flex().gap_2().children(
                [preset.soft, preset.accent, gpui::rgba(0xffffff)].map(|color| {
                    div()
                        .size(px(24.))
                        .rounded_full()
                        .border_1()
                        .border_color(colors.border)
                        .bg(color)
                }),
            ))
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(preset.name),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(preset.description),
                    ),
            )
            .on_click(move |_, _, cx| theme::select(selected_theme, cx))
    }


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
    pub(super) fn region_appearance_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("区域外观"),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .child(self.region_color_card(
                        "侧边栏",
                        "独立于应用主题",
                        &self.sidebar_color_picker,
                        &self.sidebar_opacity,
                        theme::sidebar_opacity(cx),
                        cx,
                    ))
                    .child(self.region_color_card(
                        "终端",
                        "仅修改终端容器背景",
                        &self.terminal_color_picker,
                        &self.terminal_opacity,
                        theme::terminal_opacity(cx),
                        cx,
                    )),
            )
    }

    fn region_color_card(
        &self,
        title: &'static str,
        description: &'static str,
        picker: &Entity<ColorPickerState>,
        opacity: &Entity<SliderState>,
        opacity_value: f32,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme();
        v_flex()
            .p_4()
            .gap_3()
            .rounded_xl()
            .border_1()
            .border_color(colors.border)
            .bg(colors.title_bar)
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(description),
                    ),
            )
            .child(ColorPicker::new(picker).label("背景颜色"))
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .justify_between()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child("遮罩透明度")
                            .child(format!("{:.0}%", opacity_value * 100.)),
                    )
                    .child(Slider::new(opacity).horizontal().w_full()),
            )
    }
}
