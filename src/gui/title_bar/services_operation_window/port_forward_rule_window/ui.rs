use crate::component::theme;
use gpui_kit::component::Disableable;
use gpui_kit::component::{
    ActiveTheme,
    button::{Button, ButtonVariants},
    h_flex,
    input::Input,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::PortForwardRuleWindow;

impl Render for PortForwardRuleWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors;
        v_flex()
            .size_full()
            .p_5()
            .gap_2()
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .text_color(colors.foreground)
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(if self.id.is_some() {
                        "编辑端口转发"
                    } else {
                        "新增端口转发"
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .child("新规则默认停用，保存后可在列表中启用。"),
            )
            .children(
                [
                    ("本机监听地址", &self.listen_host),
                    ("本机监听端口", &self.listen_port),
                    ("目标主机", &self.target_host),
                    ("目标端口", &self.target_port),
                ]
                .into_iter()
                .map(|(label, input)| {
                    v_flex()
                        .gap_1()
                        .child(div().text_xs().font_weight(FontWeight::MEDIUM).child(label))
                        .child(Input::new(input).disabled(self.busy))
                }),
            )
            .when_some(self.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(colors.danger_foreground)
                        .child(error),
                )
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-port-forward-rule")
                            .outline()
                            .label("取消")
                            .disabled(self.busy)
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("save-port-forward-rule")
                            .primary()
                            .label(if self.busy { "保存中…" } else { "保存" })
                            .disabled(self.busy)
                            .on_click(cx.listener(Self::save)),
                    ),
            )
    }
}
