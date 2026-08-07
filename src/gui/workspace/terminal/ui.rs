mod terminal_render {
    use std::{cell::Cell, rc::Rc, sync::Arc};

    use gpui::prelude::FluentBuilder;
    use gpui::*;
    use gpui_component::{ElementExt, h_flex, menu::ContextMenuExt};

    use crate::{
        component::theme,
        domain::terminal::{TerminalFrame, TerminalLine},
    };

    use super::super::{
        CopyTerminal, TERMINAL_FONT_FAMILY, TERMINAL_FONT_SIZE, TerminalView,
        internal::{TerminalPoint, buffer_row, nearest_character_column, selected_fragments},
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
            let colors = theme::CustomerUiTheme::colors(cx);
            let terminal_background = colors.workspace_background;
            let terminal_foreground = colors.workspace_text_color;
            let selection_background = theme::CustomerUiTheme::terminal_selection_background(cx);
            let selection_foreground = theme::CustomerUiTheme::terminal_selection_foreground(cx);
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
                let fragments =
                    selected_fragments(&line, buffer_row(&frame, index), active_selection);
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
                    .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                        let point = terminal_point(
                            index,
                            &select_line,
                            event.position,
                            select_bounds.get(),
                            cell_width,
                        );
                        let _ = select_view.update(cx, |this, cx| {
                            this.begin_text_selection(select_workspace_id.clone(), point, cx);
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
                    .child(render_gutter(
                        timestamp,
                        line_number,
                        terminal_background,
                        terminal_foreground,
                    ))
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
                            .child(render_terminal_text(
                                fragments,
                                terminal_background,
                                selection_background,
                                selection_foreground,
                                terminal_foreground,
                            )),
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
        terminal_foreground: Hsla,
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
                    .text_color(terminal_foreground)
                    .when_some(timestamp, |this, timestamp| this.child(timestamp)),
            )
            .child(
                h_flex()
                    .flex_1()
                    .justify_end()
                    .text_color(terminal_foreground)
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

    fn has_selection_contrast(foreground: Hsla, background: Hsla) -> bool {
        (foreground.l - background.l).abs() >= 0.35
    }

    fn render_terminal_text(
        fragments: Vec<super::super::internal::SelectedFragment>,
        terminal_background: Hsla,
        selection_background: Hsla,
        selection_foreground: Hsla,
        terminal_foreground: Hsla,
    ) -> StyledText {
        let mut text = String::new();
        let mut runs = Vec::with_capacity(fragments.len());
        for fragment in fragments {
            if fragment.text.is_empty() {
                continue;
            }
            let foreground = terminal_foreground;
            let foreground =
                if fragment.selected && !has_selection_contrast(foreground, selection_background) {
                    selection_foreground
                } else {
                    foreground
                };
            let background = if fragment.selected {
                selection_background
            } else {
                terminal_background
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
                color: foreground,
                background_color: Some(background),
                underline: fragment.style.underline.then_some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(foreground),
                    wavy: false,
                }),
                strikethrough: None,
            });
        }
        StyledText::new(text).with_runs(runs)
    }
}

use gpui::*;
use gpui_component::v_flex;

use crate::{
    component::{color::rgb_to_u32, theme},
    domain::terminal::{TerminalSessionCommand, TerminalStatus},
    gui::workspace::ui::render_empty_workspace,
};

use super::*;

fn terminal_cell_width(window: &Window) -> Pixels {
    let run = TextRun {
        len: 1,
        font: Font {
            family: TERMINAL_FONT_FAMILY.into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let width = window
        .text_system()
        .layout_line("0", px(TERMINAL_FONT_SIZE), &[run], None)
        .width;
    if width > Pixels::ZERO { width } else { px(8.) }
}

impl TerminalView {
    pub(super) fn render_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return render_empty_workspace().into_any_element();
        };
        let Some(terminal_model) = self.model(&workspace_id) else {
            return div().size_full().into_any_element();
        };
        self.observed_revision = Some((workspace_id.clone(), terminal_model.revision()));
        let (frame, status, message) = {
            let terminal = terminal_model.read();
            (
                terminal.frame.clone(),
                terminal.status.clone(),
                terminal.message.clone(),
            )
        };
        self.scroll_handle
            .sync(&frame, self.command_sender(&workspace_id));
        self.sync_list(&workspace_id, frame.lines.len().max(1));
        let cell_width = terminal_cell_width(window);

        let focus = self.focus.clone();
        let scroll_commands = self.command_sender(&workspace_id);
        let content = if status == TerminalStatus::Failed {
            render_connection_error(message).into_any_element()
        } else {
            self.render_terminal_list(workspace_id, frame, cell_width, cx)
                .into_any_element()
        };
        v_flex()
            .id("ssh-terminal")
            .role(Role::Terminal)
            .aria_label("SSH 终端")
            .size_full()
            .key_context(TERMINAL_KEY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::send_tab))
            .on_action(cx.listener(Self::copy_terminal))
            .on_action(cx.listener(Self::paste_terminal))
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                focus.focus(window, cx);
            })
            .on_mouse_up(MouseButton::Left, cx.listener(Self::finish_text_selection))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::finish_text_selection))
            .on_scroll_wheel(move |event, _, cx| {
                let pixels = event.delta.pixel_delta(px(TERMINAL_LINE_HEIGHT)).y;
                let lines = (f32::from(pixels) / TERMINAL_LINE_HEIGHT).round() as i32;
                if lines != 0 {
                    if let Some(commands) = &scroll_commands {
                        let _ = commands.send(TerminalSessionCommand::Scroll { lines });
                    }
                    cx.stop_propagation();
                }
            })
            .bg(theme::CustomerUiTheme::workspace_background(cx))
            .text_color(theme::CustomerUiTheme::colors(cx).workspace_text_color)
            .child(content)
            .into_any_element()
    }

    fn sync_list(&mut self, workspace_id: &str, line_count: usize) {
        if self.listed_workspace_id.as_deref() != Some(workspace_id) {
            self.list_state
                .reset_with_uniform_height(line_count, px(TERMINAL_LINE_HEIGHT));
            self.list_state.set_follow_mode(FollowMode::Tail);
            self.listed_workspace_id = Some(workspace_id.to_owned());
            return;
        }

        let previous_count = self.list_state.item_count();
        if line_count > previous_count {
            self.list_state
                .splice(previous_count..previous_count, line_count - previous_count);
        } else if line_count < previous_count {
            self.list_state
                .reset_with_uniform_height(line_count, px(TERMINAL_LINE_HEIGHT));
        }
    }

    pub(super) fn sync_pty_size(
        &mut self,
        workspace_id: &str,
        viewport: Size<Pixels>,
        cell_width: Pixels,
    ) {
        let text_width = f32::from(viewport.width)
            - terminal_render::GUTTER_WIDTH
            - internal::SCROLLBAR_WIDTH
            - TERMINAL_TEXT_HORIZONTAL_PADDING;
        let columns = (text_width / f32::from(cell_width)).floor().max(20.0) as u32;
        let rows = (f32::from(viewport.height) / TERMINAL_LINE_HEIGHT)
            .floor()
            .max(6.0) as u32;
        let pty_size = (workspace_id.to_owned(), columns, rows);
        if self.last_pty_size.as_ref() != Some(&pty_size) {
            self.resize(workspace_id, columns, rows);
            self.last_pty_size = Some(pty_size);
        }
    }
}

fn render_connection_error(message: Option<String>) -> impl IntoElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .px_8()
        .child(
            v_flex()
                .max_w(px(640.))
                .items_center()
                .gap_3()
                .rounded_lg()
                .border_1()
                .border_color(rgb_to_u32(127, 29, 29))
                .bg(rgb_to_u32(30, 18, 24))
                .px_6()
                .py_5()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb_to_u32(248, 113, 113))
                        .child("连接失败"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb_to_u32(203, 213, 225))
                        .text_center()
                        .child(message.unwrap_or_else(|| "未提供错误原因".to_owned())),
                ),
        )
}
