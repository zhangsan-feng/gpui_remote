use std::{rc::Rc, time::Duration};

use gpui::*;
use gpui_component::scroll::*;
use gpui_component::{
    menu::{ContextMenuExt, PopupMenu},
    *,
};

#[derive(Clone, Copy, Debug)]
struct ListTransition {
    from_index: usize,
    to_index: usize,
    epoch: u64,
}

impl ListTransition {
    fn displacement_direction(self, index: usize) -> Option<f32> {
        if self.from_index < self.to_index && (self.from_index..self.to_index).contains(&index) {
            Some(1.)
        } else if self.from_index > self.to_index
            && (self.to_index + 1..=self.from_index).contains(&index)
        {
            Some(-1.)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default)]
struct DraggableListState {
    dragging_id: Option<ElementId>,
    transition: Option<ListTransition>,
    transition_epoch: u64,
}

struct DraggableListItem {
    id: ElementId,
    render: Rc<dyn Fn(&App) -> AnyElement>,
}

struct DraggableListDragPreview {
    render: Rc<dyn Fn(&App) -> AnyElement>,
    width: Option<Pixels>,
    height: Option<Pixels>,
}

impl Render for DraggableListDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preview = div()
            .relative()
            .rounded_md()
            .opacity(0.82)
            .shadow_lg()
            .bg(rgb(0xffffff));
        let preview = if let Some(width) = self.width {
            preview.w(width)
        } else {
            preview
        };
        let preview = if let Some(height) = self.height {
            preview.h(height)
        } else {
            preview
        };

        preview.child((self.render)(cx))
    }
}

type MouseDownHandler = Rc<dyn Fn(ElementId, &MouseDownEvent, &mut Context<DraggableList>)>;
type ActionIdChangeHandler = Rc<dyn Fn(Option<ElementId>, &mut Context<DraggableList>)>;
type ContextMenuHandler = Rc<dyn Fn(ElementId, PopupMenu, &mut Context<PopupMenu>) -> PopupMenu>;
type ColorProvider = Rc<dyn Fn(&App) -> Rgba>;

pub struct DraggableList {
    axis: Axis,
    items: Vec<DraggableListItem>,
    state: Option<Entity<DraggableListState>>,
    view_state: DraggableListState,
    selected_id: Option<ElementId>,
    context_menu_id: Option<ElementId>,
    item_height: Pixels,
    item_width: Pixels,
    item_sizes: Rc<Vec<gpui::Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    item_bg: Rgba,
    item_selected_bg: Rgba,
    stem_hover_bg: Rgba,
    item_bg_provider: Option<ColorProvider>,
    item_selected_bg_provider: Option<ColorProvider>,
    stem_hover_bg_provider: Option<ColorProvider>,
    transition_duration: Duration,
    on_mouse_down: Option<MouseDownHandler>,
    on_action_id_change: Option<ActionIdChangeHandler>,
    on_context_menu: Option<ContextMenuHandler>,
}

impl Render for DraggableList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let instance_id = cx.entity_id();
        let state =
            window.use_keyed_state(format!("draggable-list-state-{instance_id}"), cx, |_, _| {
                DraggableListState::default()
            });
        self.state = Some(state.clone());

        self.view_state = state.read(cx).clone();

        self.render_virtualized(cx)
    }
}

impl DraggableList {
    pub fn new() -> Self {
        Self {
            axis: Axis::Vertical,
            items: Vec::new(),
            state: None,
            view_state: DraggableListState::default(),
            selected_id: None,
            context_menu_id: None,
            item_height: px(0.),
            item_width: px(0.),
            item_sizes: Rc::new(Vec::new()),
            scroll_handle: VirtualListScrollHandle::new(),
            item_bg: rgb(0xffffff),
            item_selected_bg: rgb(0xdbeafe),
            stem_hover_bg: rgb(0xf3f4f6),
            item_bg_provider: None,
            item_selected_bg_provider: None,
            stem_hover_bg_provider: None,
            transition_duration: Duration::from_millis(180),
            on_mouse_down: None,
            on_action_id_change: None,
            on_context_menu: None,
        }
    }

    pub fn child<E, F>(&mut self, id: impl Into<ElementId>, render: F) -> &mut Self
    where
        E: IntoElement,
        F: Fn() -> E + 'static,
    {
        self.child_with_context(id, move |_| render())
    }

    pub fn child_with_context<E, F>(&mut self, id: impl Into<ElementId>, render: F) -> &mut Self
    where
        E: IntoElement,
        F: Fn(&App) -> E + 'static,
    {
        let id = id.into();

        self.items.push(DraggableListItem {
            id,
            render: Rc::new(move |cx| render(cx).into_any_element()),
        });
        self.rebuild_item_sizes();
        self
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) -> &mut Self {
        self.items.clear();
        self.rebuild_item_sizes();
        self.selected_id = None;
        self.context_menu_id = None;
        if let Some(state) = self.state.clone() {
            state.update(cx, |state, _| state.clear_drag());
        }
        cx.notify();
        self
    }

    pub fn remove(&mut self, id: &ElementId, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.items.iter().position(|item| &item.id == id) else {
            return false;
        };

        self.items.remove(index);
        self.rebuild_item_sizes();

        if self.selected_id.as_ref() == Some(id) {
            self.set_selected_id_value(None, cx);
        }
        if self.context_menu_id.as_ref() == Some(id) {
            self.context_menu_id = None;
        }
        if let Some(state) = self.state.clone() {
            state.update(cx, |state, _| state.clear_drag());
        }
        cx.notify();
        true
    }

    pub fn set_selected_id(&mut self, id: &ElementId, cx: &mut Context<Self>) -> &mut Self {
        self.set_selected_id_value(Some(id.clone()), cx);
        self
    }

    pub fn selected_id(&self) -> Option<&ElementId> {
        self.selected_id.as_ref()
    }

    pub fn on_action_id_change<F>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(Option<ElementId>, &mut Context<DraggableList>) + 'static,
    {
        self.on_action_id_change = Some(Rc::new(handler));
        self
    }

    fn set_selected_id_value(&mut self, id: Option<ElementId>, cx: &mut Context<Self>) {
        if self.selected_id == id {
            return;
        }

        self.selected_id = id.clone();
        if let Some(handler) = self.on_action_id_change.clone() {
            handler(id, cx);
        }
        cx.notify();
    }

    pub fn on_mouse_down<F>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(ElementId, &MouseDownEvent, &mut Context<DraggableList>) + 'static,
    {
        self.on_mouse_down = Some(Rc::new(handler));
        self
    }

    pub(crate) fn set_context_menu<F>(&mut self, handler: F) -> &mut Self
    where
        F: Fn(ElementId, PopupMenu, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    {
        self.on_context_menu = Some(Rc::new(handler));
        self
    }

    pub fn set_item_height(&mut self, height: Pixels) -> &mut Self {
        if self.item_height == height {
            return self;
        }
        self.item_height = height;
        self.rebuild_item_sizes();
        self
    }
    pub fn set_item_width(&mut self, width: Pixels) -> &mut Self {
        if self.item_width == width {
            return self;
        }
        self.item_width = width;
        self.rebuild_item_sizes();
        self
    }

    pub fn set_item_bg(&mut self, color: Rgba) -> &mut Self {
        self.item_bg = color;
        self.item_bg_provider = None;
        self
    }

    pub fn set_item_selected_bg(&mut self, color: Rgba) -> &mut Self {
        self.item_selected_bg = color;
        self.item_selected_bg_provider = None;
        self
    }

    pub fn set_item_hover_bg(&mut self, color: Rgba) -> &mut Self {
        self.stem_hover_bg = color;
        self.stem_hover_bg_provider = None;
        self
    }

    pub fn set_item_bg_provider<F>(&mut self, provider: F) -> &mut Self
    where
        F: Fn(&App) -> Rgba + 'static,
    {
        self.item_bg_provider = Some(Rc::new(provider));
        self
    }

    pub fn set_item_selected_bg_provider<F>(&mut self, provider: F) -> &mut Self
    where
        F: Fn(&App) -> Rgba + 'static,
    {
        self.item_selected_bg_provider = Some(Rc::new(provider));
        self
    }

    pub fn set_item_hover_bg_provider<F>(&mut self, provider: F) -> &mut Self
    where
        F: Fn(&App) -> Rgba + 'static,
    {
        self.stem_hover_bg_provider = Some(Rc::new(provider));
        self
    }

    /// Sets how long neighbouring items take to settle into their new slots.
    pub fn set_transition_duration(&mut self, duration: Duration) -> &mut Self {
        self.transition_duration = duration;
        self
    }

    pub fn set_axis(&mut self, axis: Axis) -> &mut Self {
        if self.axis == axis {
            return self;
        }
        self.axis = axis;
        self
    }

    fn render_virtualized(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity();
        let virtual_list_id = format!("draggable-list-virtual-{}", cx.entity_id());
        let item_sizes = self.item_sizes.clone();

        let list = match self.axis {
            Axis::Horizontal => v_flex()
                .size_full()
                .child(
                    h_virtual_list(
                        view.clone(),
                        virtual_list_id.clone(),
                        item_sizes.clone(),
                        move |this, range, _, cx| {
                            range
                                .filter_map(|index| {
                                    let item = this.items.get(index)?;
                                    Some(this.render_virtual_item(
                                        index,
                                        item.id.clone(),
                                        item.render.clone(),
                                        cx,
                                    ))
                                })
                                .collect::<Vec<_>>()
                        },
                    )
                    .track_scroll(&self.scroll_handle)
                    .into_any_element(),
                )
                .child(
                    div().w_full().h(px(10.)).child(
                        Scrollbar::vertical(&self.scroll_handle)
                            .mode(ScrollbarMode::Always)
                            .axis(ScrollbarAxis::Horizontal),
                    ),
                )
                .into_any_element(),
            Axis::Vertical => h_flex()
                .size_full()
                .child(
                    v_virtual_list(
                        view.clone(),
                        virtual_list_id,
                        item_sizes,
                        move |this, range, _, cx| {
                            range
                                .filter_map(|index| {
                                    let item = this.items.get(index)?;
                                    Some(this.render_virtual_item(
                                        index,
                                        item.id.clone(),
                                        item.render.clone(),
                                        cx,
                                    ))
                                })
                                .collect::<Vec<_>>()
                        },
                    )
                    .track_scroll(&self.scroll_handle)
                    .into_any_element(),
                )
                .child(
                    div().w(px(10.)).h_full().child(
                        Scrollbar::vertical(&self.scroll_handle)
                            .mode(ScrollbarMode::Always)
                            .axis(ScrollbarAxis::Vertical),
                    ),
                )
                .into_any_element(),
        };

        let mut container = div()
            .id("draggable-list-context-menu")
            .size_full()
            .child(list);

        if self.axis == Axis::Horizontal {
            let scroll_handle = self.scroll_handle.clone();
            container = container.on_scroll_wheel(move |event, window, _cx| {
                let delta = event.delta.pixel_delta(window.line_height());
                let delta_x = if delta.x.is_zero() { delta.y } else { delta.x };
                if !delta_x.is_zero() {
                    let mut offset = scroll_handle.offset();
                    offset.x += delta_x;
                    scroll_handle.set_offset(offset);
                }
            });
        }

        if let Some(handler) = self.on_context_menu.clone() {
            container
                .context_menu(move |menu, _window, menu_cx| {
                    let Some(id) = view.read(menu_cx).context_menu_id.clone() else {
                        return menu;
                    };
                    handler(id, menu, menu_cx)
                })
                .into_any_element()
        } else {
            container.into_any_element()
        }
    }

    fn rebuild_item_sizes(&mut self) {
        self.item_sizes = Rc::new(
            (0..self.items.len())
                .map(|_| size(self.item_width, self.item_height))
                .collect(),
        );
    }

    fn render_virtual_item(
        &mut self,
        visible_index: usize,
        id: ElementId,
        render: Rc<dyn Fn(&App) -> AnyElement>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.selected_id.as_ref() == Some(&id);
        let is_dragging = self.view_state.dragging_id.as_ref() == Some(&id);
        let transition = self.view_state.transition.and_then(|transition| {
            transition
                .displacement_direction(visible_index)
                .map(|direction| (transition, direction))
        });
        let item_bg = self
            .item_bg_provider
            .as_ref()
            .map(|provider| provider(cx))
            .unwrap_or(self.item_bg);
        let selected_bg = self
            .item_selected_bg_provider
            .as_ref()
            .map(|provider| provider(cx))
            .unwrap_or(self.item_selected_bg);
        let hover_bg = self
            .stem_hover_bg_provider
            .as_ref()
            .map(|provider| provider(cx))
            .unwrap_or(self.stem_hover_bg);

        let click_id = id.clone();
        let context_menu_id = id.clone();
        let drag_id = id.clone();
        let drag_target_id = id.clone();
        let axis = self.axis;
        let state = self.state.clone();
        let drag_state = state.clone();
        let item_width = self.item_width;

        let drag_preview_render = render.clone();
        let drag_preview_width = match (axis, item_width.as_f32() == 0.0) {
            (Axis::Horizontal, true) => None,
            (_, true) => Some(px(240.)),
            (_, false) => Some(item_width),
        };
        let drag_preview_height = match (axis, self.item_height.as_f32() == 0.0) {
            (Axis::Horizontal, true) => None,
            (_, true) => Some(px(40.)),
            (_, false) => Some(self.item_height),
        };

        let child = div()
            .id(format!("draggable-list-item-{id}"))
            .relative()
            // .h(self.item_height)
            .bg(if is_dragging {
                rgb(0xe5e7eb)
            } else if selected {
                selected_bg
            } else {
                item_bg
            })
            .hover(move |this| this.bg(hover_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    this.set_selected_id_value(Some(click_id.clone()), cx);
                    if let Some(handler) = this.on_mouse_down.clone() {
                        handler(click_id.clone(), event, cx);
                    }
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _, _, cx| {
                    this.context_menu_id = Some(context_menu_id.clone());
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener({
                    let state = state.clone();
                    move |_, _, _, cx| {
                        if let Some(state) = &state {
                            state.update(cx, |state, _| state.clear_drag());
                        }
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener({
                    let state = state.clone();
                    move |_, _, _, cx| {
                        if let Some(state) = &state {
                            state.update(cx, |state, _| state.clear_drag());
                        }
                    }
                }),
            )
            .on_drag(drag_id, move |_, _, _, cx| {
                // A previous drag may have ended outside the item, leaving its
                // transition behind. Reset it before starting a new drag so the
                // previous animation cannot leak into this one.
                if let Some(state) = drag_state.clone() {
                    state.update(cx, |state, _| state.clear_drag());
                }
                let render = drag_preview_render.clone();
                cx.new(|_| DraggableListDragPreview {
                    render,
                    width: drag_preview_width,
                    height: drag_preview_height,
                })
            })
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<ElementId>, _, cx| {
                    let Some(state) = state.clone() else {
                        return;
                    };
                    let dragged_item_id = event.drag(cx).clone();
                    if !event.bounds.contains(&event.event.position) {
                        return;
                    }
                    // The dragged item itself is not an insertion target. After a
                    // downward reorder its recycled callback can otherwise process
                    // the same pointer position a second time.
                    if dragged_item_id == drag_target_id {
                        return;
                    }
                    // A reorder rebuilds the virtual-list items. Ignore a move event
                    // delivered to an old item closure whose index is no longer valid;
                    // otherwise a downward drag can apply the same transition twice.
                    if this
                        .items
                        .get(visible_index)
                        .map(|item| item.id != drag_target_id)
                        .unwrap_or(true)
                    {
                        return;
                    }

                    let (position, midpoint) = match axis {
                        Axis::Horizontal => (
                            event.event.position.x,
                            event.bounds.origin.x + event.bounds.size.width / 2.,
                        ),
                        Axis::Vertical => (
                            event.event.position.y,
                            event.bounds.origin.y + event.bounds.size.height / 2.,
                        ),
                    };
                    let insertion_index = if position < midpoint {
                        visible_index
                    } else {
                        visible_index + 1
                    };

                    let Some(from_index) = this
                        .items
                        .iter()
                        .position(|item| item.id == dragged_item_id)
                    else {
                        return;
                    };
                    if insertion_index == from_index || insertion_index == from_index + 1 {
                        // The pointer is still in the current slot. Avoid writing the same
                        // state on every drag-move event, which can restart/reconcile the
                        // transition and make vertical dragging visibly jump.
                        return;
                    }

                    let to_index = if from_index < insertion_index {
                        insertion_index - 1
                    } else {
                        insertion_index
                    };
                    if state
                        .read(cx)
                        .transition
                        .is_some_and(|transition| transition.to_index == to_index)
                    {
                        return;
                    }
                    if from_index < this.items.len() && insertion_index <= this.items.len() {
                        let item = this.items.remove(from_index);
                        this.items.insert(to_index, item);
                    }
                    state.update(cx, |state, _| {
                        state.transition_epoch = state.transition_epoch.wrapping_add(1);
                        state.transition = Some(ListTransition {
                            from_index,
                            to_index,
                            epoch: state.transition_epoch,
                        });
                        state.dragging_id = Some(dragged_item_id.clone());
                    });
                    cx.notify();
                }),
            )
            .child(render(cx));

        let child = match self.axis {
            Axis::Vertical => child.w_full(),
            Axis::Horizontal if item_width.as_f32() != 0.0 => child.w(item_width),
            Axis::Horizontal => child,
        };

        let root = div()
            .id(format!("draggable-list-item-root-{id}"))
            .relative()
            // .h(self.item_height)
            .child(child);

        let root = if let Some((transition, direction)) = transition {
            let axis = self.axis;
            let distance = match axis {
                Axis::Horizontal => self.item_width,
                Axis::Vertical => self.item_height,
            };
            let start_offset = distance * direction;

            root.with_animation(
                format!("draggable-list-transition-{id}-{}", transition.epoch),
                Animation::new(self.transition_duration).with_easing(ease_in_out),
                move |element, delta| {
                    let offset = px(start_offset.as_f32() * (1. - delta));
                    match axis {
                        Axis::Horizontal => element.left(offset),
                        Axis::Vertical => element.top(offset),
                    }
                },
            )
            .into_any_element()
        } else {
            root.into_any_element()
        };

        match self.axis {
            Axis::Vertical => div().w_full().child(root).into_any_element(),
            Axis::Horizontal if item_width.as_f32() != 0.0 => {
                div().w(item_width).child(root).into_any_element()
            }
            Axis::Horizontal => root,
        }
    }
}

impl DraggableListState {
    fn clear_drag(&mut self) {
        self.dragging_id = None;
        self.transition = None;
    }
}
