use std::{cell::RefCell, rc::Rc};

use gpui_kit::*;

#[derive(Clone, Debug, PartialEq)]
struct WindowSnapshot {
    native_bounds: Bounds<Pixels>,
    viewport: Size<Pixels>,
    layout: Bounds<Pixels>,
    scale: f32,
    maximized: bool,
}

#[derive(Default)]
pub(super) struct WindowDiagnostics {
    last_paint: Rc<RefCell<Option<WindowSnapshot>>>,
}

pub(super) fn log_window_bounds(window: &Window) {
    log::info!(
        "main_window_bounds_changed: native_bounds={:?}, viewport={:?}, scale={}, maximized={}",
        window.bounds(),
        window.viewport_size(),
        window.scale_factor(),
        window.is_maximized(),
    );
}

impl WindowDiagnostics {
    pub(super) fn layout_probe(&self) -> impl IntoElement {
        let last_paint = self.last_paint.clone();
        // An absolute, non-interactive probe measures the root without changing its layout.
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let snapshot = WindowSnapshot {
                    native_bounds: window.bounds(),
                    viewport: window.viewport_size(),
                    layout: bounds,
                    scale: window.scale_factor(),
                    maximized: window.is_maximized(),
                };
                let mut previous = last_paint.borrow_mut();
                if previous.as_ref() == Some(&snapshot) {
                    return;
                }
                let covers_viewport =
                    bounds.origin == point(px(0.), px(0.)) && bounds.size == snapshot.viewport;
                log::info!(
                    "main_window_paint_geometry: covers_viewport={}, {:?}",
                    covers_viewport,
                    snapshot,
                );
                *previous = Some(snapshot);
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }
}
