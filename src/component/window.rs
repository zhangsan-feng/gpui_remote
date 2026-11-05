use gpui_kit::*;

pub fn window_center(window: &mut Window, window_size: Size<Pixels>) -> WindowBounds {
    let w = window_size.width.as_f32();
    let h = window_size.height.as_f32();
    let parent_bounds = window.bounds();
    let parent_x = parent_bounds.origin.x;
    let parent_y = parent_bounds.origin.y;

    let parent_width = parent_bounds.size.width;
    let parent_height = parent_bounds.size.height;

    let child_x = parent_x + (parent_width - px(w)) / 2.0;
    let child_y = parent_y + (parent_height - px(h)) / 2.0;
    let window_size = size(px(w), px(h));

    let bounds = Bounds {
        origin: Point {
            x: child_x,
            y: child_y,
        },
        size: window_size,
    };

    WindowBounds::Windowed(bounds)
}

pub fn window_center_options(window: &mut Window, window_size: Size<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(window_center(window, window_size)),
        window_min_size: Some(window_size),
        is_resizable: false,
        kind: WindowKind::Dialog,
        focus: true,
        titlebar: Some(TitlebarOptions {
            title: Some("".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}
