use std::panic::Location;

use gpui_kit::component::scroll::{Scrollbar, ScrollbarMode};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

#[derive(IntoElement)]
pub(super) struct ActiveConnectionLog {
    id: ElementId,
    entries: Vec<String>,
    empty_message: &'static str,
    muted_foreground: Hsla,
}

#[track_caller]
pub(super) fn active_connection_log(
    entries: Vec<String>,
    empty_message: &'static str,
    cx: &App,
) -> ActiveConnectionLog {
    ActiveConnectionLog {
        id: ElementId::CodeLocation(*Location::caller()),
        entries,
        empty_message,
        muted_foreground: cx.theme().colors.muted_foreground,
    }
}

impl RenderOnce for ActiveConnectionLog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, _| ScrollHandle::default())
            .read(cx)
            .clone();
        let entries = if self.entries.is_empty() {
            vec![self.empty_message.to_owned()]
        } else {
            self.entries
        };

        div()
            .w_full()
            .max_h(px(112.))
            .relative()
            .child(
                v_flex()
                    .id((self.id.clone(), "scroll-area"))
                    .w_full()
                    .max_h(px(112.))
                    .min_h_0()
                    .pr_3()
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .children(
                        entries
                            .into_iter()
                            .map(|entry| log_line(entry, self.muted_foreground)),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .w(px(10.))
                    .child(
                        Scrollbar::vertical(&scroll_handle)
                            .mode(ScrollbarMode::Always)
                            .viewport_from_layout(),
                    ),
            )
    }
}

fn log_line(text: impl Into<SharedString>, foreground: Hsla) -> Div {
    let text = text.into();
    div().w_full().text_xs().text_color(foreground).child(text)
}
