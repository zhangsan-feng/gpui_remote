use crate::component::theme::{self, AppTheme};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::color_picker::{ColorPicker, ColorPickerState};
use gpui_component::slider::Slider;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, ThemeColor,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputState},
    scroll::ScrollableElement,
    v_flex,
};

use super::{SettingsOperationWindow, SettingsSection};

impl SettingsOperationWindow {
    pub(super) fn render_view(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = theme::CustomerUiTheme::colors(cx);
        let content = match self.active_section {
            SettingsSection::Theme => self.theme_section(cx).into_any_element(),
            SettingsSection::Wallpaper => self.wallpaper_section(cx).into_any_element(),
            SettingsSection::Mcp => self.mcp_section(cx).into_any_element(),
        };

        h_flex()
            .size_full()
            .items_stretch()
            .bg(colors.background)
            .text_color(colors.text_color)
            .child(self.sidebar(cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .bg(Hsla::transparent_black())
                    .child(
                        v_flex()
                            .flex_1()
                            .overflow_y_scrollbar()
                            .p_6()
                            .gap_5()
                            .child(content),
                    ),
            )
    }

    fn sidebar(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(184.))
            .flex_shrink_0()
            .p_3()
            .gap_2()
            .border_r_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::sidebar_background(cx))
            .child(
                div()
                    .px_3()
                    .pt_2()
                    .pb_3()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("系统配置"),
            )
            .child(self.sidebar_item(SettingsSection::Theme, IconName::Palette, "主题", cx))
            .child(self.sidebar_item(SettingsSection::Wallpaper, IconName::File, "背景图片", cx))
            .child(self.sidebar_item(SettingsSection::Mcp, IconName::Settings2, "MCP 服务", cx))
    }

    fn sidebar_item(
        &self,
        section: SettingsSection,
        icon: IconName,
        label: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let ui_colors = theme::CustomerUiTheme::colors(cx);
        let selected = self.active_section == section;
        h_flex()
            .id(match section {
                SettingsSection::Theme => "settings-section-theme",
                SettingsSection::Wallpaper => "settings-section-wallpaper",
                SettingsSection::Mcp => "settings-section-mcp",
            })
            .h(px(38.))
            .px_3()
            .gap_2()
            .rounded_lg()
            .when(selected, |this| {
                this.bg(ui_colors.select_background)
                    .text_color(ui_colors.text_color)
            })
            .when(!selected, |this| {
                this.text_color(cx.theme().sidebar_foreground)
                    .hover(|style| style.bg(ui_colors.hover_background))
            })
            .cursor_pointer()
            .child(Icon::new(icon).small())
            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(label))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.active_section = section;
                cx.notify();
            }))
    }

    fn theme_section(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_5()
            .child(self.section_heading("主题", "选择默认主题或自定义配色。", cx))
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("主题模式"),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_3()
                            .children(AppTheme::BUILT_IN.map(|theme| {
                                self.theme_card(theme, theme.label(), theme.description(), cx)
                            })),
                    ),
            )
            .child(self.custom_color_panel(cx))
            .child(self.color_overrides_panel(cx))
    }

    fn wallpaper_section(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_5()
            .child(self.section_heading("背景图片", "设置应用背景图片和透明度。", cx))
            .child(self.wallpaper_panel(cx))
    }

    fn mcp_section(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_5()
            .child(self.section_heading("MCP 服务", "配置 MCP 服务监听地址和访问令牌。", cx))
            .child(self.mcp_panel(cx))
    }

    fn mcp_panel(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        v_flex()
            .p_4()
            .gap_3()
            .rounded_xl()
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("服务状态"),
                            )
                            .child(div().text_xs().text_color(colors.muted_foreground).child(
                                if self.mcp_enabled {
                                    "应用启动时自动运行，保存配置后会重启服务"
                                } else {
                                    "服务已关闭，保存配置后会停止 MCP 服务"
                                },
                            )),
                    )
                    .child(
                        Button::new("toggle-mcp-enabled")
                            .when(self.mcp_enabled, |this| this.primary())
                            .when(!self.mcp_enabled, |this| this.outline())
                            .label(if self.mcp_enabled {
                                "已启动"
                            } else {
                                "已停止"
                            })
                            .on_click(cx.listener(Self::toggle_mcp_enabled)),
                    ),
            )
            .child(Self::mcp_field("Host", &self.mcp_host, colors))
            .child(Self::mcp_field("Port", &self.mcp_port, colors))
            .child(self.mcp_token_field(cx))
            .child(
                h_flex().justify_end().pt_2().child(
                    Button::new("apply-mcp-settings")
                        .primary()
                        .label("保存并重启")
                        .on_click(cx.listener(Self::apply_mcp_settings)),
                ),
            )
            .when_some(self.mcp_error.clone(), |this, error| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(colors.danger_foreground)
                        .child(error),
                )
            })
    }

    fn mcp_field(label: &'static str, input: &Entity<InputState>, colors: ThemeColor) -> Div {
        h_flex()
            .w_full()
            .h(px(34.))
            .gap_2()
            .items_center()
            .child(
                div()
                    .w(px(80.))
                    .flex_shrink_0()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.muted_foreground)
                    .child(label),
            )
            .child(div().flex_1().child(Input::new(input).small()))
    }

    fn mcp_token_field(&self, cx: &Context<Self>) -> Div {
        let colors = cx.theme().colors;
        h_flex()
            .w_full()
            .h(px(34.))
            .gap_2()
            .items_center()
            .child(
                div()
                    .w(px(80.))
                    .flex_shrink_0()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.muted_foreground)
                    .child("Token"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(30.))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .border_1()
                    .border_color(theme::CustomerUiTheme::border_color(cx))
                    .bg(theme::CustomerUiTheme::panel_background(cx))
                    .text_xs()
                    .text_color(colors.foreground)
                    .truncate()
                    .child(self.mcp_token.clone()),
            )
            .child(
                Button::new("copy-mcp-token")
                    .outline()
                    .small()
                    .label("复制")
                    .on_click(cx.listener(Self::copy_mcp_token)),
            )
    }

    fn section_heading(
        &self,
        title: &'static str,
        description: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(description),
            )
    }

    fn custom_color_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let selected = theme::CustomerTheme::active(cx) == AppTheme::Custom;
        let featured_colors = vec![theme::CustomerTheme::preview(AppTheme::Wisteria).accent];

        h_flex()
            .p_4()
            .gap_4()
            .justify_between()
            .rounded_xl()
            .border_1()
            .border_color(if selected {
                colors.primary
            } else {
                theme::CustomerUiTheme::border_color(cx)
            })
            .bg(theme::CustomerUiTheme::panel_background(cx))
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
                                    .child("自定义配色"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child("选择一个主色，自动生成背景、边框与交互颜色。"),
                    ),
            )
            .child(
                ColorPicker::new(&self.color_picker)
                    .label("主色")
                    .featured_colors(featured_colors),
            )
    }

    fn color_overrides_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_4()
            .rounded_xl()
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .child(self.color_override_row(
                "字体颜色",
                "应用内文字的主要颜色",
                &self.font_color_picker,
                theme::CustomerUiColor::font_color(cx).is_some(),
                "reset-font-color",
                Self::reset_font_color,
                cx,
            ))
            .child(self.color_override_row(
                "背景颜色",
                "应用主题的基础背景色",
                &self.background_color_picker,
                theme::CustomerUiColor::background_color(cx).is_some(),
                "reset-background-color",
                Self::reset_background_color,
                cx,
            ))
            .child(self.color_override_row(
                "悬浮颜色",
                "鼠标悬浮在列表项上的背景色",
                &self.hover_color_picker,
                theme::CustomerUiColor::hover_color(cx).is_some(),
                "reset-hover-color",
                Self::reset_hover_color,
                cx,
            ))
            .child(self.color_override_row(
                "选中颜色",
                "列表项和文本选中的背景色",
                &self.selected_color_picker,
                theme::CustomerUiColor::selected_color(cx).is_some(),
                "reset-selected-color",
                Self::reset_selected_color,
                cx,
            ))
    }

    fn color_override_row(
        &self,
        title: &'static str,
        description: &'static str,
        picker: &Entity<ColorPickerState>,
        has_override: bool,
        reset_id: &'static str,
        reset: fn(&mut Self, &ClickEvent, &mut Window, &mut Context<Self>),
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme();
        h_flex()
            .items_center()
            .justify_between()
            .gap_4()
            .py_2()
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(description),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(ColorPicker::new(picker).small())
                    .when(has_override, |this| {
                        this.child(
                            Button::new(reset_id)
                                .ghost()
                                .small()
                                .label("跟随主题")
                                .on_click(cx.listener(reset)),
                        )
                    }),
            )
    }

    fn theme_card(
        &self,
        selected_theme: AppTheme,
        title: &'static str,
        description: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let component_colors = cx.theme();
        let colors = theme::CustomerUiTheme::colors(cx);
        let preview = theme::CustomerTheme::preview(selected_theme);
        let selected = theme::CustomerTheme::active(cx) == selected_theme;

        v_flex()
            .id(format!("theme-{selected_theme:?}"))
            .relative()
            .h(px(138.))
            .p_4()
            .gap_3()
            .rounded_xl()
            .border_1()
            .border_color(if selected {
                component_colors.primary
            } else {
                theme::CustomerUiTheme::border_color(cx)
            })
            .bg(if selected {
                colors.select_background
            } else {
                theme::CustomerUiTheme::panel_background(cx)
            })
            .hover(|style| style.bg(colors.hover_background))
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
                        .bg(component_colors.primary)
                        .text_color(component_colors.primary_foreground)
                        .child(Icon::new(IconName::Check).xsmall()),
                )
            })
            .child(h_flex().gap_2().children(
                [preview.hover, preview.accent, preview.background].map(|color| {
                    div()
                        .size(px(24.))
                        .rounded_full()
                        .border_1()
                        .border_color(theme::CustomerUiTheme::border_color(cx))
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
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(component_colors.muted_foreground)
                            .child(description),
                    ),
            )
            .on_click(move |_, _, cx| theme::CustomerTheme::select(selected_theme, cx))
    }

    fn wallpaper_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let wallpaper = theme::CustomerUiColor::wallpaper(cx);
        let wallpaper_name = wallpaper
            .as_ref()
            .and_then(|(path, _)| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("未选择图片")
            .to_owned();

        v_flex()
            .p_4()
            .gap_3()
            .rounded_xl()
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .child(
                h_flex()
                    .gap_3()
                    .justify_between()
                    .child(
                        v_flex()
                            .gap_1()
                            .min_w_0()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Icon::new(IconName::File).small())
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(wallpaper_name)
                                            .truncate(),
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
                            .child(format!(
                                "{:.0}%",
                                theme::CustomerUiColor::wallpaper_opacity(cx) * 100.
                            )),
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
            })
    }
}
