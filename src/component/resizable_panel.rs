use crate::component::color::rgb_to_u32;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{h_flex, v_flex};
use uuid::Uuid;

#[derive(Clone)]
struct PanelResizeHandle {
    owner: EntityId,
}

impl Render for PanelResizeHandle {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone, Copy)]
pub struct ResizeHandleStyle {
    pub size: Pixels,
    pub bg: Hsla,
    pub active_bg: Hsla,
}

impl Default for ResizeHandleStyle {
    fn default() -> Self {
        Self {
            size: px(2.0),
            bg: rgb_to_u32(220, 220, 220).into(),
            active_bg: rgb(0x007acc).into(),
        }
    }
}

pub struct ResizablePanel {
    first_panel: AnyView,
    second_panel: AnyView,
    axis: Axis,
    panel_size: f32,
    min_panel_size: f32,
    max_panel_size: f32,
    resize_handle_style: ResizeHandleStyle,
    resize_handle_id: ElementId,
    resize_start: Option<(Point<Pixels>, f32)>,
}

impl ResizablePanel {
    pub fn new(
        first_panel: impl Into<AnyView>,
        second_panel: impl Into<AnyView>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            first_panel: first_panel.into(),
            second_panel: second_panel.into(),
            axis: Axis::Horizontal,
            panel_size: 260.0,
            min_panel_size: 150.0,
            max_panel_size: 600.0,
            resize_handle_style: ResizeHandleStyle::default(),
            resize_handle_id: ElementId::Uuid(Uuid::new_v4()),
            resize_start: None,
        }
    }

    pub fn with_axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn with_panel_size(mut self, size: f32) -> Self {
        self.panel_size = size;
        self
    }

    pub fn with_panel_size_range(mut self, min_size: f32, max_size: f32) -> Self {
        self.min_panel_size = min_size;
        self.max_panel_size = max_size;
        self
    }

    pub fn with_resize_handle_style(mut self, style: ResizeHandleStyle) -> Self {
        self.resize_handle_style = style;
        self
    }

    pub fn set_id(mut self, id: impl Into<ElementId>) -> Self {
        self.resize_handle_id = id.into();
        self
    }

    fn handle_resize(&mut self, event: &DragMoveEvent<PanelResizeHandle>, cx: &mut Context<Self>) {
        let PanelResizeHandle { owner } = event.drag(cx);
        if *owner != cx.entity_id() {
            return;
        }

        let Some((start_position, initial_size)) = self.resize_start else {
            return;
        };

        let delta = match self.axis {
            Axis::Horizontal => event.event.position.x - start_position.x,
            Axis::Vertical => event.event.position.y - start_position.y,
        };
        self.panel_size = panel_size_after_drag(
            initial_size,
            f32::from(delta),
            self.min_panel_size,
            self.max_panel_size,
        );
        cx.notify();
    }

    fn render_resize_handle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let style = self.resize_handle_style;

        div()
            .id(self.resize_handle_id.clone())
            .bg(style.bg)
            .active(move |this| this.bg(style.active_bg))
            .when(matches!(self.axis, Axis::Horizontal), |this| {
                this.w(style.size).h_full().cursor_col_resize()
            })
            .when(matches!(self.axis, Axis::Vertical), |this| {
                this.h(style.size).w_full().cursor_row_resize()
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _window, _cx| {
                    this.resize_start = Some((event.position, this.panel_size));
                }),
            )
            .on_drag(
                PanelResizeHandle {
                    owner: cx.entity_id(),
                },
                |handle, _, _, app| {
                    app.stop_propagation();
                    app.new(|_| handle.clone())
                },
            )
            .on_drag_move(cx.listener(|this, event, _window, cx| {
                this.handle_resize(event, cx);
            }))
    }
}

fn panel_size_after_drag(initial_size: f32, delta: f32, min_size: f32, max_size: f32) -> f32 {
    (initial_size + delta).clamp(min_size, max_size)
}

impl Render for ResizablePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let container = match self.axis {
            Axis::Horizontal => h_flex(),
            Axis::Vertical => v_flex(),
        };

        container
            .size_full()
            .overflow_hidden()
            .child(
                div()
                    .when(matches!(self.axis, Axis::Horizontal), |this| {
                        this.w(px(self.panel_size)).h_full()
                    })
                    .when(matches!(self.axis, Axis::Vertical), |this| {
                        this.h(px(self.panel_size)).w_full()
                    })
                    .overflow_hidden()
                    .child(self.first_panel.clone()),
            )
            .child(self.render_resize_handle(cx))
            .child(
                div()
                    .flex_1()
                    .when(matches!(self.axis, Axis::Horizontal), |this| this.h_full())
                    .when(matches!(self.axis, Axis::Vertical), |this| this.w_full())
                    .overflow_hidden()
                    .child(self.second_panel.clone()),
            )
    }
}
