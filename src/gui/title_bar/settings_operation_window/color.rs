use gpui::*;
use gpui_component::{
    ActiveTheme,
    color_picker::{ColorPicker, ColorPickerState},
    h_flex,
    slider::{Slider, SliderState},
    v_flex,
};

use crate::component::theme;

use super::SettingsOperationWindow;

impl SettingsOperationWindow {
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
