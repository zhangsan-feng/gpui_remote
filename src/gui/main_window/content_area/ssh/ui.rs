use gpui_kit::component::ActiveTheme;

mod highlight;

mod terminal_render {
    use std::{cell::Cell, rc::Rc, sync::Arc};

    use gpui_kit::component::{ActiveTheme, ElementExt, ThemeColor, h_flex, menu::ContextMenuExt};
    use gpui_kit::prelude::FluentBuilder;
    use gpui_kit::*;

    use crate::{
        domain::terminal::{TerminalColor, TerminalCursor, TerminalFrame, TerminalLine},
        gui::component::color::rgb_to_u32,
    };

    use super::super::{
        CopyTerminal, TERMINAL_FONT_FAMILY, TerminalView,
        internal::{TerminalPoint, buffer_row, nearest_character_column, selected_fragments},
    };
    use super::highlight::{Highlight, frame_highlights};

    pub(super) const GUTTER_WIDTH: f32 = 116.0;

    // ANSI order: black, red, green, yellow, blue, magenta, cyan, white, then bright variants.
    const ANSI_LIGHT: [u32; 16] = [
        0x18181b, 0xb91c1c, 0x15803d, 0x854d0e, 0x1d4ed8, 0xa21caf, 0x0e7490, 0xd4d4d8, 0x52525b,
        0xdc2626, 0x047857, 0xa16207, 0x2563eb, 0xc026d3, 0x0f766e, 0xffffff,
    ];
    const ANSI_DARK: [u32; 16] = [
        0x18181b, 0xf87171, 0x4ade80, 0xfacc15, 0x60a5fa, 0xe879f9, 0x22d3ee, 0xe4e4e7, 0xa1a1aa,
        0xfca5a5, 0x86efac, 0xfde047, 0x93c5fd, 0xf0abfc, 0x67e8f9, 0xffffff,
    ];

    struct TerminalTextPalette {
        foreground: Hsla,
        background: Hsla,
        canvas: Hsla,
        ansi: &'static [u32; 16],
        selection_foreground: Hsla,
        selection_background: Hsla,
        selection_canvas: Hsla,
    }

    impl TerminalTextPalette {
        fn resolve(&self, color: TerminalColor, default_background: Hsla) -> Hsla {
            match color {
                TerminalColor::DefaultForeground => self.foreground,
                TerminalColor::DimForeground => self.foreground.opacity(0.67),
                TerminalColor::DefaultBackground => default_background,
                TerminalColor::Ansi { index, dimmed } => {
                    let color: Hsla = rgb(self.ansi[usize::from(index)]).into();
                    if dimmed { color.opacity(0.67) } else { color }
                }
                TerminalColor::Rgb(rgb) => rgb_to_u32(rgb.red, rgb.green, rgb.blue).into(),
            }
        }
    }

    impl TerminalView {
        pub(super) fn render_terminal_list(
            &self,
            workspace_id: String,
            frame: Arc<TerminalFrame>,
            cell_width: Pixels,
            cx: &mut Context<Self>,
        ) -> impl IntoElement {
            let colors = cx.theme().colors;
            let gutter_border = colors.border;
            let terminal_cursor_color = colors.caret;
            let selection_canvas = colors.background.blend(colors.selection);
            let text_palette = TerminalTextPalette {
                foreground: colors.foreground,
                background: crate::gui::main_window::content_area::ui::workspace_background(cx),
                canvas: colors.background,
                ansi: if colors.background.l < 0.5 {
                    &ANSI_DARK
                } else {
                    &ANSI_LIGHT
                },
                selection_background: colors.selection,
                selection_canvas,
                selection_foreground: if selection_canvas.l < 0.5 {
                    ThemeColor::dark().foreground
                } else {
                    ThemeColor::light().foreground
                },
            };
            let selection = self.selection.clone();
            let terminal_view = cx.weak_entity();
            let resize_view = terminal_view.clone();
            let resize_workspace_id = workspace_id.clone();
            let menu_workspace_id = workspace_id.clone();
            let right_click_workspace_id = workspace_id.clone();
            let highlights = frame_highlights(&frame);
            let font_size = px(f32::from(self.font_size));
            let line_height = px(self.line_height());
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
                let highlights = highlights.get(index).map_or(&[][..], Vec::as_slice);
                let cursor = frame.cursor.filter(|cursor| cursor.row == index);
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

                let element = h_flex()
                    .h(line_height)
                    .w_full()
                    .font_family(TERMINAL_FONT_FAMILY)
                    .text_size(font_size)
                    .line_height(line_height)
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
                        text_palette.background,
                        text_palette.foreground,
                        gutter_border,
                    ))
                    .child(
                        h_flex()
                            .relative()
                            .h_full()
                            .flex_1()
                            .min_w_0()
                            .pl_2()
                            .pr_3()
                            .cursor_text()
                            .overflow_hidden()
                            .on_prepaint(move |bounds, _, _| text_bounds_writer.set(bounds))
                            .child(render_terminal_text(fragments, highlights, &text_palette))
                            .when_some(cursor, |this, cursor| {
                                this.child(render_terminal_cursor(
                                    cursor,
                                    cell_width,
                                    terminal_cursor_color,
                                ))
                            }),
                    )
                    .into_any_element();
                element
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
                .child(capture_terminal_zoom(cx))
                .on_mouse_down(MouseButton::Right, move |event, _, _| {
                    let _span = tracing::debug_span!("terminal_context_menu_mouse_down",
                        workspace_id = %right_click_workspace_id,
                        position = ?event.position,
                        can_copy,
                    )
                    .entered();
                    tracing::debug!("right mouse button pressed");
                })
                .on_prepaint(move |bounds, _, cx| {
                    let _ = resize_view.update(cx, |this, cx| {
                        this.sync_pty_size(&resize_workspace_id, bounds.size, cell_width, cx);
                    });
                })
                .context_menu(move |menu, _, _| {
                    let _span = tracing::debug_span!("terminal_context_menu_build",
                        workspace_id = %menu_workspace_id, can_copy,
                    )
                    .entered();
                    tracing::debug!("terminal_context_menu_build_started");
                    let menu = if can_copy {
                        menu.menu("复制", Box::new(CopyTerminal))
                    } else {
                        menu
                    };
                    tracing::debug!("terminal_context_menu_build_finished");
                    menu
                })
        }
    }

    fn capture_terminal_zoom(cx: &Context<TerminalView>) -> impl IntoElement {
        let terminal = cx.weak_entity();
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hitbox, window, _| {
                // Capture Ctrl+wheel before the list changes its scroll position.
                window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
                    if phase == DispatchPhase::Capture
                        && event.modifiers.control
                        && hitbox.should_handle_scroll(window)
                    {
                        let delta = event.delta.pixel_delta(px(1.)).y;
                        if delta != Pixels::ZERO {
                            let _ =
                                terminal.update(cx, |this, cx| this.zoom(delta > Pixels::ZERO, cx));
                            cx.stop_propagation();
                        }
                    }
                });
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }

    fn render_gutter(
        timestamp: Option<SharedString>,
        line_number: Option<SharedString>,
        terminal_background: Hsla,
        terminal_foreground: Hsla,
        gutter_border: Hsla,
    ) -> Div {
        h_flex()
            .h_full()
            .w(px(GUTTER_WIDTH))
            .flex_shrink_0()
            .px_2()
            .gap_1()
            .justify_between()
            .border_r_1()
            .border_color(gutter_border)
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

    fn render_terminal_cursor(cursor: TerminalCursor, cell_width: Pixels, color: Hsla) -> Div {
        let left = 8. + f32::from(cell_width) * cursor.column as f32;
        div()
            .absolute()
            .left(px(left))
            .top(px(2.))
            .bottom(px(2.))
            .w(cell_width)
            .bg(color.opacity(0.88))
            .border_1()
            .border_color(color)
    }

    fn render_terminal_text(
        fragments: Vec<super::super::internal::SelectedFragment>,
        highlights: &[Highlight],
        palette: &TerminalTextPalette,
    ) -> StyledText {
        let mut text = String::new();
        let mut runs = Vec::with_capacity(fragments.len());
        for fragment in fragments {
            if fragment.text.is_empty() {
                continue;
            }
            let background = if fragment.selected {
                palette.selection_background
            } else {
                palette.resolve(fragment.style.background, palette.background)
            };
            let start = text.len();
            text.push_str(&fragment.text);
            let end = text.len();
            let foreground = palette.resolve(fragment.style.foreground, palette.canvas);
            let font = Font {
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
            };
            let mut push_run = |len, color| {
                if len == 0 {
                    return;
                }
                let foreground = if fragment.style.hidden {
                    Hsla::transparent_black()
                } else if fragment.selected
                    && !has_selection_contrast(color, palette.selection_canvas)
                {
                    palette.selection_foreground
                } else {
                    color
                };
                runs.push(TextRun {
                    len,
                    font: font.clone(),
                    color: foreground,
                    background_color: Some(background),
                    underline: fragment.style.underline.then_some(UnderlineStyle {
                        thickness: px(1.),
                        color: Some(foreground),
                        wavy: false,
                    }),
                    strikethrough: None,
                });
            };
            let mut position = start;
            if !fragment.style.hidden
                && fragment.style.foreground == TerminalColor::DefaultForeground
                && fragment.style.background == TerminalColor::DefaultBackground
            {
                let first = highlights.partition_point(|highlight| highlight.end <= start);
                for highlight in &highlights[first..] {
                    if highlight.start >= end {
                        break;
                    }
                    let highlight_start = position.max(highlight.start);
                    let highlight_end = end.min(highlight.end);
                    push_run(highlight_start - position, foreground);
                    push_run(
                        highlight_end - highlight_start,
                        rgb(palette.ansi[highlight.color as usize]).into(),
                    );
                    position = highlight_end;
                }
            }
            push_run(end - position, foreground);
        }
        StyledText::new(text).with_runs(runs)
    }
}

use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::domain::terminal::TerminalStatus;

use super::{super::ui::render_empty_workspace, *};

fn terminal_cell_width(window: &Window, font_size: f32) -> Pixels {
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
        .layout_line("0", px(font_size), &[run], None)
        .width;
    if width > Pixels::ZERO {
        width
    } else {
        px(font_size * 0.6)
    }
}

impl TerminalView {
    pub(super) fn render_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return render_empty_workspace(cx).into_any_element();
        };
        let Some(terminal_model) = self.model(&workspace_id) else {
            return div().size_full().into_any_element();
        };
        self.observed_gui_snapshot_version =
            Some((workspace_id.clone(), terminal_model.gui_snapshot_version()));
        let (frame, status, message) = {
            let terminal = terminal_model.read();
            (
                terminal.frame.clone(),
                terminal.status.clone(),
                terminal.message.clone(),
            )
        };
        if self.focus_pending && status != TerminalStatus::Failed {
            self.focus_pending = false;
            let focus = self.focus.clone();
            window.on_next_frame(move |window, cx| {
                focus.focus(window, cx);
            });
        }
        self.scroll_handle.sync(&frame);
        self.sync_list(&workspace_id, frame.lines.len().max(1));
        let cell_width = terminal_cell_width(window, f32::from(self.font_size));
        let line_height = self.line_height();

        let focus = self.focus.clone();
        let scroll_workspace_id = workspace_id.clone();
        let scroll_application = crate::application::APPLICATION.clone();
        let content = if status == TerminalStatus::Failed {
            render_connection_error(message, cx).into_any_element()
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
            .on_scroll(move |event, _, cx| {
                let pixels = event.delta.pixel_delta(px(line_height)).y;
                let lines = (f32::from(pixels) / line_height).round() as i32;
                if lines != 0 {
                    let application = scroll_application.clone();
                    let workspace_id = scroll_workspace_id.clone();
                    cx.spawn(async move |_cx| {
                        if let Err(error) = application.ssh.scroll(&workspace_id, lines).await {
                            log::debug!("滚动 SSH 终端失败: {error}");
                        }
                    })
                    .detach();
                    cx.stop_propagation();
                }
            })
            .bg(crate::gui::main_window::content_area::ui::workspace_background(cx))
            .text_color(cx.theme().colors.foreground)
            .child(content)
            .into_any_element()
    }

    fn sync_list(&mut self, workspace_id: &str, line_count: usize) {
        let line_height = px(self.line_height());
        if self.listed_workspace_id.as_deref() != Some(workspace_id) {
            self.list_state
                .reset_with_uniform_height(line_count, line_height);
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
                .reset_with_uniform_height(line_count, line_height);
        }
    }

    pub(super) fn sync_pty_size(
        &mut self,
        workspace_id: &str,
        viewport: Size<Pixels>,
        cell_width: Pixels,
        cx: &mut Context<Self>,
    ) {
        let text_width = f32::from(viewport.width)
            - terminal_render::GUTTER_WIDTH
            - internal::SCROLLBAR_WIDTH
            - TERMINAL_TEXT_HORIZONTAL_PADDING;
        let columns = (text_width / f32::from(cell_width)).floor().max(20.0) as u32;
        let rows = (f32::from(viewport.height) / self.line_height())
            .floor()
            .max(6.0) as u32;
        let pty_size = (workspace_id.to_owned(), columns, rows);
        if self.last_pty_size.as_ref() != Some(&pty_size) {
            let application = crate::application::APPLICATION.clone();
            let workspace_id = workspace_id.to_owned();
            cx.spawn(async move |_this, _cx| {
                if let Err(error) = application.ssh.resize(&workspace_id, columns, rows).await {
                    log::debug!("调整 SSH 终端大小失败: {error}");
                }
            })
            .detach();
            self.last_pty_size = Some(pty_size);
        }
    }
}

fn render_connection_error(message: Option<String>, cx: &App) -> impl IntoElement {
    let colors = &cx.theme().colors;
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
                .border_color(colors.danger.opacity(0.35))
                .bg(colors.group_box)
                .px_6()
                .py_5()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.danger)
                        .child("连接失败"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(colors.foreground)
                        .text_center()
                        .child(message.unwrap_or_else(|| "未提供错误原因".to_owned())),
                ),
        )
}
