use std::{cell::Cell, rc::Rc, sync::Arc};

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{ElementExt, h_flex, menu::ContextMenuExt};

use crate::{
    component::color::rgb_to_u32,
    domain::terminal::{TerminalFrame, TerminalLine, TerminalRgb},
};

use super::{
    CopyTerminal, TerminalView,
    selection::{TerminalPoint, line_width, selected_fragments},
};

pub(super) const GUTTER_WIDTH: f32 = 116.0;
const CELL_WIDTH: f32 = 8.0;
const TEXT_PADDING: f32 = 8.0;

impl TerminalView {
    pub(super) fn render_terminal_list(
        &self,
        workspace_id: String,
        frame: Arc<TerminalFrame>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = self.selection.clone();
        let terminal_view = cx.weak_entity();
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
                .font_family("Consolas")
                .text_size(px(13.))
                .line_height(px(19.))
                .whitespace_nowrap()
                .child(render_gutter(timestamp, line_number))
                .child(
                    h_flex()
                        .h_full()
                        .flex_1()
                        .min_w_0()
                        .pl_2()
                        .pr_3()
                        .overflow_hidden()
                        .on_prepaint(move |bounds, _, _| text_bounds_writer.set(bounds))
                        .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                            let point = terminal_point(
                                index,
                                &select_line,
                                event.position,
                                select_bounds.get(),
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
                                );
                                let _ = extend_view.update(cx, |this, cx| {
                                    this.extend_selection(&extend_workspace_id, point, cx);
                                });
                            }
                        })
                        .children(fragments.into_iter().map(|fragment| {
                            let foreground = terminal_color(fragment.style.foreground);
                            let background = if fragment.selected {
                                rgb_to_u32(51, 65, 85)
                            } else {
                                terminal_color(fragment.style.background)
                            };
                            div()
                                .h_full()
                                .text_color(foreground)
                                .bg(background)
                                .when(fragment.style.bold, |this| {
                                    this.font_weight(FontWeight::BOLD)
                                })
                                .when(fragment.style.italic, |this| this.italic())
                                .when(fragment.style.underline, |this| this.underline())
                                .child(fragment.text)
                        })),
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
            .context_menu(move |menu, _, _| {
                if can_copy {
                    menu.menu("复制", Box::new(CopyTerminal))
                } else {
                    menu
                }
            })
    }
}

fn render_gutter(timestamp: Option<SharedString>, line_number: Option<SharedString>) -> Div {
    h_flex()
        .h_full()
        .w(px(GUTTER_WIDTH))
        .flex_shrink_0()
        .px_2()
        .gap_1()
        .justify_between()
        .border_r_1()
        .border_color(rgb_to_u32(48, 43, 55))
        .bg(rgb_to_u32(17, 16, 20))
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
) -> TerminalPoint {
    let local_x = (f32::from(position.x - bounds.origin.x) - TEXT_PADDING).max(0.0);
    let clicked_column = (local_x / CELL_WIDTH).floor() as usize;
    TerminalPoint {
        row,
        column: clicked_column.min(line_width(line).saturating_sub(1)),
    }
}

fn terminal_color(color: TerminalRgb) -> Rgba {
    rgb_to_u32(color.red, color.green, color.blue)
}
