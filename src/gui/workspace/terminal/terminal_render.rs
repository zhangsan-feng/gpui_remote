use std::{cell::Cell, rc::Rc, sync::Arc};

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{ElementExt, h_flex, menu::ContextMenuExt};

use crate::{
    component::{color::rgb_to_u32, theme},
    domain::terminal::{TerminalFrame, TerminalLine, TerminalRgb},
};

use super::{
    CopyTerminal, TERMINAL_FONT_FAMILY, TERMINAL_FONT_SIZE, TerminalView,
    selection::{TerminalPoint, nearest_character_column, selected_fragments},
};

pub(super) const GUTTER_WIDTH: f32 = 116.0;

impl TerminalView {
    pub(super) fn render_terminal_list(
        &self,
        workspace_id: String,
        frame: Arc<TerminalFrame>,
        cell_width: Pixels,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let terminal_background = theme::terminal_color(cx);
        let selection = self.selection.clone();
        let terminal_view = cx.weak_entity();
        let resize_view = terminal_view.clone();
        let resize_workspace_id = workspace_id.clone();
        let menu_workspace_id = workspace_id.clone();
        let terminal_list = list(self.list_state.clone(), move |index, _, _| {
            let line = frame.lines.get(index).cloned().unwrap_or_default();
            let timestamp = line.timestamp.clone().map(SharedString::from);
            let line_number = line
                .number
                .map(|number| SharedString::from(number.to_string()));
            let active_selection = selection
                .as_ref()
                .filter(|selection| selection.workspace_id == workspace_id);
            let fragments = selected_fragments(&line, index, active_selection);
            let text_bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
            let text_bounds_writer = text_bounds.clone();
            let select_bounds = text_bounds.clone();
            let extend_bounds = text_bounds.clone();
            let select_line = line.clone();
            let extend_line = line.clone();
            let select_view = terminal_view.clone();
            let extend_view = terminal_view.clone();
            let select_workspace_id = workspace_id.clone();
            let extend_workspace_id = workspace_id.clone();

            h_flex()
                .h(px(19.))
                .w_full()
                .font_family(TERMINAL_FONT_FAMILY)
                .text_size(px(TERMINAL_FONT_SIZE))
                .line_height(px(19.))
                .whitespace_nowrap()
                .child(render_gutter(timestamp, line_number, terminal_background))
                .child(
                    h_flex()
                        .h_full()
                        .flex_1()
                        .min_w_0()
                        .pl_2()
                        .pr_3()
                        .cursor_text()
                        .overflow_hidden()
                        .on_prepaint(move |bounds, _, _| text_bounds_writer.set(bounds))
                        .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                            let point = terminal_point(
                                index,
                                &select_line,
                                event.position,
                                select_bounds.get(),
                                cell_width,
                            );
                            let _ = select_view.update(cx, |this, cx| {
                                this.select_point(
                                    select_workspace_id.clone(),
                                    point,
                                    event.modifiers.shift,
                                    cx,
                                );
                            });
                        })
                        .on_mouse_move(move |event, _, cx| {
                            if event.dragging() {
                                let point = terminal_point(
                                    index,
                                    &extend_line,
                                    event.position,
                                    extend_bounds.get(),
                                    cell_width,
                                );
                                let _ = extend_view.update(cx, |this, cx| {
                                    this.extend_selection(&extend_workspace_id, point, cx);
                                });
                            }
                        })
                        .child(render_terminal_text(fragments, terminal_background)),
                )
                .into_any_element()
        })
        .size_full();
        let can_copy = self
            .selection
            .as_ref()
            .is_some_and(|selection| selection.workspace_id == menu_workspace_id);

        div()
            .id("terminal-scroll-area")
            .relative()
            .flex_1()
            .w_full()
            .overflow_hidden()
            .child(terminal_list)
            .child(self.render_scrollbar(cx))
            .on_prepaint(move |bounds, _, cx| {
                let _ = resize_view.update(cx, |this, _| {
                    this.sync_pty_size(&resize_workspace_id, bounds.size, cell_width);
                });
            })
            .context_menu(move |menu, _, _| {
                if can_copy {
                    menu.menu("复制", Box::new(CopyTerminal))
                } else {
                    menu
                }
            })
    }
}

fn render_gutter(
    timestamp: Option<SharedString>,
    line_number: Option<SharedString>,
    terminal_background: Hsla,
) -> Div {
    h_flex()
        .h_full()
        .w(px(GUTTER_WIDTH))
        .flex_shrink_0()
        .px_2()
        .gap_1()
        .justify_between()
        .border_r_1()
        .border_color(terminal_background)
        .bg(terminal_background)
        .text_size(px(11.))
        .child(
            div()
                .w(px(68.))
                .text_color(rgb_to_u32(100, 116, 139))
                .when_some(timestamp, |this, timestamp| this.child(timestamp)),
        )
        .child(
            h_flex()
                .flex_1()
                .justify_end()
                .text_color(rgb_to_u32(71, 85, 105))
                .when_some(line_number, |this, line_number| this.child(line_number)),
        )
}

fn terminal_point(
    row: usize,
    line: &TerminalLine,
    position: Point<Pixels>,
    bounds: Bounds<Pixels>,
    cell_width: Pixels,
) -> TerminalPoint {
    let local_x = f32::from(position.x - bounds.origin.x).max(0.0);
    let clicked_column = (local_x / f32::from(cell_width)).floor() as usize;
    TerminalPoint {
        row,
        column: nearest_character_column(line, clicked_column),
    }
}

fn terminal_color(color: TerminalRgb) -> Rgba {
    rgb_to_u32(color.red, color.green, color.blue)
}

fn render_terminal_text(
    fragments: Vec<super::selection::SelectedFragment>,
    terminal_background: Hsla,
) -> StyledText {
    let mut text = String::new();
    let mut runs = Vec::with_capacity(fragments.len());
    for fragment in fragments {
        if fragment.text.is_empty() {
            continue;
        }
        let foreground = terminal_color(fragment.style.foreground);
        let background: Hsla = if fragment.selected {
            rgb_to_u32(51, 65, 85).into()
        } else if fragment.style.background == super::buffer::default_background() {
            terminal_background
        } else {
            terminal_color(fragment.style.background).into()
        };
        let len = fragment.text.len();
        text.push_str(&fragment.text);
        runs.push(TextRun {
            len,
            font: Font {
                family: TERMINAL_FONT_FAMILY.into(),
                weight: if fragment.style.bold {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                },
                style: if fragment.style.italic {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                },
                ..Default::default()
            },
            color: foreground.into(),
            background_color: Some(background),
            underline: fragment.style.underline.then_some(UnderlineStyle {
                thickness: px(1.),
                color: Some(foreground.into()),
                wavy: false,
            }),
            strikethrough: None,
        });
    }
    StyledText::new(text).with_runs(runs)
}
