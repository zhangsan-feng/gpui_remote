use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, color_picker::ColorPicker, h_flex, v_flex,
};

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
}
