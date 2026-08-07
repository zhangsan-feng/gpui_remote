mod keyboard {
    use gpui::*;

    use super::super::{
        PasteTerminal, SendTab, TerminalView,
        core::{encode_control_key, encode_special_key},
    };

    impl TerminalView {
        pub(in crate::gui::workspace::terminal) fn key_down(
            &mut self,
            event: &KeyDownEvent,
            _: &mut Window,
            cx: &mut Context<Self>,
        ) {
            let Some(workspace_id) = self.selected_workspace_id.clone() else {
                return;
            };
            let terminal_model = self.model(&workspace_id);
            let application_cursor = terminal_model
                .as_ref()
                .is_some_and(|model| model.read().frame.application_cursor);
            if let Some(bytes) = encode_keystroke(&event.keystroke, application_cursor) {
                self.send_input(&workspace_id, bytes);
                cx.stop_propagation();
            }
        }

        pub(in crate::gui::workspace::terminal) fn send_tab(
            &mut self,
            _: &SendTab,
            _: &mut Window,
            _: &mut Context<Self>,
        ) {
            self.send_action_input(b"\t");
        }

        pub(in crate::gui::workspace::terminal) fn paste_terminal(
            &mut self,
            _: &PasteTerminal,
            _: &mut Window,
            cx: &mut Context<Self>,
        ) {
            let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
                return;
            };
            self.send_action_input(text.as_bytes());
            cx.stop_propagation();
        }

        fn send_action_input(&self, bytes: &[u8]) {
            let Some(workspace_id) = self.selected_workspace_id.as_deref() else {
                return;
            };
            self.send_input(workspace_id, bytes.to_vec());
        }
    }

    fn encode_keystroke(keystroke: &Keystroke, application_cursor: bool) -> Option<Vec<u8>> {
        if let Some(sequence) = encode_special_key(&keystroke.key, application_cursor) {
            return Some(sequence.as_bytes().to_vec());
        }
        if keystroke.modifiers.control {
            return Some(vec![encode_control_key(&keystroke.key)?]);
        }

        let text = keystroke.key_char.as_ref()?;
        let mut bytes = Vec::with_capacity(text.len() + usize::from(keystroke.modifiers.alt));
        if keystroke.modifiers.alt {
            bytes.push(0x1b);
        }
        bytes.extend_from_slice(text.as_bytes());
        Some(bytes)
    }
}

mod scroll {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use gpui::*;
    use gpui_component::ElementExt;
    use tokio::sync::mpsc;

    use crate::{
        component::{color::rgb_to_u32, theme},
        domain::terminal::{TerminalFrame, TerminalSessionCommand},
    };

    use super::super::TerminalView;

    pub(in crate::gui::workspace::terminal) const SCROLLBAR_WIDTH: f32 = 16.0;

    #[derive(Default)]
    struct TerminalScrollState {
        history_size: usize,
        viewport_lines: usize,
        display_offset: usize,
        commands: Option<mpsc::UnboundedSender<TerminalSessionCommand>>,
    }

    #[derive(Clone, Default)]
    pub(in crate::gui::workspace::terminal) struct TerminalScrollHandle(
        Rc<RefCell<TerminalScrollState>>,
    );

    #[derive(Clone)]
    struct TerminalScrollbarDrag {
        owner: EntityId,
        bounds: Rc<Cell<Bounds<Pixels>>>,
        thumb_size: f32,
    }

    impl Render for TerminalScrollbarDrag {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    impl TerminalScrollHandle {
        pub(in crate::gui::workspace::terminal) fn sync(
            &self,
            frame: &TerminalFrame,
            commands: Option<mpsc::UnboundedSender<TerminalSessionCommand>>,
        ) {
            *self.0.borrow_mut() = TerminalScrollState {
                history_size: frame.history_size,
                viewport_lines: frame.lines.len(),
                display_offset: frame.display_offset,
                commands,
            };
        }

        fn thumb(&self) -> (f32, f32) {
            let state = self.0.borrow();
            let total_lines = state.history_size + state.viewport_lines;
            if state.history_size == 0 || total_lines == 0 {
                return (0.0, 1.0);
            }
            let thumb_size = (state.viewport_lines as f32 / total_lines as f32).clamp(0.08, 1.0);
            let lines_from_top = state.history_size.saturating_sub(state.display_offset);
            let progress = lines_from_top as f32 / state.history_size as f32;
            (progress * (1.0 - thumb_size), thumb_size)
        }

        fn scroll_to_progress(&self, progress: f32) {
            let mut state = self.0.borrow_mut();
            let lines_from_top =
                (progress.clamp(0.0, 1.0) * state.history_size as f32).round() as usize;
            let display_offset = state.history_size.saturating_sub(lines_from_top);
            state.display_offset = display_offset;
            if let Some(commands) = &state.commands {
                let _ = commands.send(TerminalSessionCommand::ScrollTo {
                    offset: display_offset,
                });
            }
        }
    }

    impl TerminalView {
        pub(in crate::gui::workspace::terminal) fn render_scrollbar(
            &self,
            cx: &mut Context<Self>,
        ) -> impl IntoElement {
            let (thumb_top, thumb_size) = self.scroll_handle.thumb();
            let terminal_background = theme::CustomerUiTheme::workspace_background(cx);
            let bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
            let bounds_writer = bounds.clone();
            let down_bounds = bounds.clone();
            let down_view = cx.weak_entity();

            div()
                .id("terminal-scrollbar")
                .absolute()
                .right_0()
                .top_0()
                .bottom_0()
                .w(px(SCROLLBAR_WIDTH))
                .cursor_pointer()
                .border_l_1()
                .border_color(terminal_background)
                .bg(terminal_background)
                .on_prepaint(move |scrollbar_bounds, _, _| bounds_writer.set(scrollbar_bounds))
                .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                    let progress =
                        scrollbar_progress(event.position, down_bounds.get(), thumb_size);
                    let _ = down_view.update(cx, |this, cx| {
                        this.selecting_text = false;
                        this.scroll_handle.scroll_to_progress(progress);
                        cx.notify();
                    });
                    cx.stop_propagation();
                })
                .on_drag(
                    TerminalScrollbarDrag {
                        owner: cx.entity_id(),
                        bounds,
                        thumb_size,
                    },
                    |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    },
                )
                .on_drag_move(cx.listener(Self::drag_scrollbar))
                .child(
                    div()
                        .absolute()
                        .left(px(4.))
                        .right(px(4.))
                        .top(relative(thumb_top))
                        .h(relative(thumb_size))
                        .rounded_full()
                        .bg(rgb_to_u32(113, 128, 150)),
                )
        }

        fn drag_scrollbar(
            &mut self,
            event: &DragMoveEvent<TerminalScrollbarDrag>,
            _: &mut Window,
            cx: &mut Context<Self>,
        ) {
            let drag = event.drag(cx);
            if drag.owner != cx.entity_id() {
                return;
            }
            self.selecting_text = false;
            let progress =
                scrollbar_progress(event.event.position, drag.bounds.get(), drag.thumb_size);
            self.scroll_handle.scroll_to_progress(progress);
            cx.notify();
        }
    }

    fn scrollbar_progress(position: Point<Pixels>, bounds: Bounds<Pixels>, thumb_size: f32) -> f32 {
        let height = f32::from(bounds.size.height).max(1.0);
        let pointer = (f32::from(position.y - bounds.origin.y) / height).clamp(0.0, 1.0);
        ((pointer - thumb_size / 2.0) / (1.0 - thumb_size).max(0.001)).clamp(0.0, 1.0)
    }
}

mod selection {
    use gpui::*;
    use std::sync::Arc;
    use unicode_width::UnicodeWidthChar;

    use crate::domain::terminal::{TerminalFrame, TerminalLine, TerminalStyle};

    use super::super::{CopyTerminal, TerminalView};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub(in crate::gui::workspace::terminal) struct TerminalPoint {
        pub(in crate::gui::workspace::terminal) row: usize,
        pub(in crate::gui::workspace::terminal) column: usize,
    }

    #[derive(Clone)]
    pub(in crate::gui::workspace::terminal) struct TerminalSelection {
        pub(in crate::gui::workspace::terminal) workspace_id: String,
        anchor: TerminalPoint,
        head: TerminalPoint,
        frame: Arc<TerminalFrame>,
    }

    pub(in crate::gui::workspace::terminal) struct SelectedFragment {
        pub(in crate::gui::workspace::terminal) text: String,
        pub(in crate::gui::workspace::terminal) style: TerminalStyle,
        pub(in crate::gui::workspace::terminal) selected: bool,
    }

    impl TerminalSelection {
        fn frame_start_row(&self) -> usize {
            self.frame
                .history_size
                .saturating_sub(self.frame.display_offset)
        }

        fn range(&self) -> (TerminalPoint, TerminalPoint) {
            if self.anchor <= self.head {
                (self.anchor, self.head)
            } else {
                (self.head, self.anchor)
            }
        }

        fn contains(&self, point: TerminalPoint) -> bool {
            let (start, end) = self.range();
            start <= point && point <= end
        }
    }

    impl TerminalView {
        pub(in crate::gui::workspace::terminal) fn copy_terminal(
            &mut self,
            _: &CopyTerminal,
            _: &mut Window,
            cx: &mut Context<Self>,
        ) {
            let Some(selection) = &self.selection else {
                return;
            };
            let text = selected_text(&selection.frame, selection);
            if !text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            cx.stop_propagation();
        }

        pub(in crate::gui::workspace::terminal) fn begin_text_selection(
            &mut self,
            workspace_id: String,
            viewport_point: TerminalPoint,
            cx: &mut Context<Self>,
        ) {
            let frame = self
                .model(&workspace_id)
                .map(|model| model.read().frame.clone())
                .unwrap_or_default();
            let point = buffer_point(&frame, viewport_point);
            self.selection = None;
            self.selection_origin = Some((workspace_id, point));
            self.selecting_text = false;
            cx.notify();
        }

        pub(in crate::gui::workspace::terminal) fn extend_selection(
            &mut self,
            workspace_id: &str,
            viewport_point: TerminalPoint,
            cx: &mut Context<Self>,
        ) {
            if !self.selecting_text {
                let Some((origin_workspace_id, anchor)) = self.selection_origin.as_ref() else {
                    return;
                };
                if origin_workspace_id != workspace_id {
                    return;
                }
                let frame = self
                    .model(workspace_id)
                    .map(|model| model.read().frame.clone())
                    .unwrap_or_default();
                let point = buffer_point(&frame, viewport_point);
                if *anchor == point {
                    return;
                }
                self.selection = Some(TerminalSelection {
                    workspace_id: workspace_id.to_owned(),
                    anchor: *anchor,
                    head: point,
                    frame,
                });
                self.selecting_text = true;
                cx.notify();
                return;
            }
            let frame = self
                .model(workspace_id)
                .map(|model| model.read().frame.clone())
                .unwrap_or_default();
            let point = buffer_point(&frame, viewport_point);
            if let Some(selection) = self
                .selection
                .as_mut()
                .filter(|selection| selection.workspace_id == workspace_id)
            {
                if selection.head != point {
                    selection.head = point;
                    cx.notify();
                }
            }
        }

        pub(in crate::gui::workspace::terminal) fn finish_text_selection(
            &mut self,
            _: &MouseUpEvent,
            _: &mut Window,
            _: &mut Context<Self>,
        ) {
            self.selection_origin = None;
            self.selecting_text = false;
        }
    }

    pub(in crate::gui::workspace::terminal) fn selected_fragments(
        line: &TerminalLine,
        row: usize,
        selection: Option<&TerminalSelection>,
    ) -> Vec<SelectedFragment> {
        let selection = selection.filter(|selection| {
            let (start, end) = selection.range();
            start.row <= row && row <= end.row
        });
        if selection.is_none() {
            return line
                .spans
                .iter()
                .map(|span| SelectedFragment {
                    text: span.text.clone(),
                    style: span.style.clone(),
                    selected: false,
                })
                .collect();
        }

        let mut fragments = Vec::<SelectedFragment>::new();
        let mut column = 0;
        for span in &line.spans {
            for character in span.text.chars() {
                let selected = selection
                    .is_some_and(|selection| selection.contains(TerminalPoint { row, column }));
                if let Some(fragment) = fragments.last_mut().filter(|fragment| {
                    fragment.style == span.style && fragment.selected == selected
                }) {
                    fragment.text.push(character);
                } else {
                    fragments.push(SelectedFragment {
                        text: character.to_string(),
                        style: span.style.clone(),
                        selected,
                    });
                }
                column += terminal_character_width(character);
            }
        }
        fragments
    }

    pub(in crate::gui::workspace::terminal) fn nearest_character_column(
        line: &TerminalLine,
        target: usize,
    ) -> usize {
        let mut column = 0;
        for character in line.spans.iter().flat_map(|span| span.text.chars()) {
            let width = terminal_character_width(character);
            if width > 0 && target < column + width {
                return column;
            }
            column += width;
        }
        column.saturating_sub(1)
    }

    fn selected_text(frame: &TerminalFrame, selection: &TerminalSelection) -> String {
        let (start, end) = selection.range();
        let frame_start_row = selection.frame_start_row();
        let start_index = start.row.saturating_sub(frame_start_row);
        let end_index = end.row.saturating_sub(frame_start_row);
        let Some(lines) = frame
            .lines
            .get(start_index..=end_index.min(frame.lines.len().saturating_sub(1)))
        else {
            return String::new();
        };
        let mut text = String::new();
        for (line_offset, line) in lines.iter().enumerate() {
            let row = start_index + frame_start_row + line_offset;
            let mut column = 0;
            for character in line.spans.iter().flat_map(|span| span.text.chars()) {
                if selection.contains(TerminalPoint { row, column }) {
                    text.push(character);
                }
                column += terminal_character_width(character);
            }
            if line_offset + 1 < lines.len() && !line.wrapped {
                text.push('\n');
            }
        }
        text
    }

    pub(in crate::gui::workspace::terminal) fn buffer_row(
        frame: &TerminalFrame,
        viewport_row: usize,
    ) -> usize {
        frame
            .history_size
            .saturating_sub(frame.display_offset)
            .saturating_add(viewport_row)
    }

    fn buffer_point(frame: &TerminalFrame, viewport_point: TerminalPoint) -> TerminalPoint {
        TerminalPoint {
            row: buffer_row(frame, viewport_point.row),
            column: viewport_point.column,
        }
    }

    fn terminal_character_width(character: char) -> usize {
        UnicodeWidthChar::width(character).unwrap_or(0)
    }
}

mod watcher {
    use gpui::Context;

    use super::super::TerminalView;

    impl TerminalView {
        pub(in crate::gui::workspace::terminal) fn start_model_watcher(
            &self,
            cx: &mut Context<Self>,
        ) {
            let terminal_updates = self.updates.clone();
            cx.spawn(async move |this, cx| {
                loop {
                    terminal_updates.notified().await;
                    if this
                        .update(cx, |this, cx| this.notify_if_model_changed(cx))
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
    }
}

pub(super) use scroll::{SCROLLBAR_WIDTH, TerminalScrollHandle};
pub(super) use selection::{
    SelectedFragment, TerminalPoint, TerminalSelection, buffer_row, nearest_character_column,
    selected_fragments,
};
