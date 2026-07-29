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

use super::TerminalView;

pub(super) const SCROLLBAR_WIDTH: f32 = 16.0;

#[derive(Default)]
struct TerminalScrollState {
    history_size: usize,
    viewport_lines: usize,
    display_offset: usize,
    commands: Option<mpsc::UnboundedSender<TerminalSessionCommand>>,
}

#[derive(Clone, Default)]
pub(super) struct TerminalScrollHandle(Rc<RefCell<TerminalScrollState>>);

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
    pub(super) fn sync(
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
    pub(super) fn render_scrollbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (thumb_top, thumb_size) = self.scroll_handle.thumb();
        let terminal_background = theme::terminal_color(cx);
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
                let progress = scrollbar_progress(event.position, down_bounds.get(), thumb_size);
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
        let progress = scrollbar_progress(event.event.position, drag.bounds.get(), drag.thumb_size);
        self.scroll_handle.scroll_to_progress(progress);
        cx.notify();
    }
}

fn scrollbar_progress(position: Point<Pixels>, bounds: Bounds<Pixels>, thumb_size: f32) -> f32 {
    let height = f32::from(bounds.size.height).max(1.0);
    let pointer = (f32::from(position.y - bounds.origin.y) / height).clamp(0.0, 1.0);
    ((pointer - thumb_size / 2.0) / (1.0 - thumb_size).max(0.001)).clamp(0.0, 1.0)
}
