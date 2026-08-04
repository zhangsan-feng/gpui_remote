use gpui::*;

pub fn window_center_options( window: &mut Window, w:f32, h:f32) -> WindowOptions {
    let parent_bounds = window.bounds();
    let parent_x = parent_bounds.origin.x;
    let parent_y = parent_bounds.origin.y;

    let parent_width = parent_bounds.size.width;
    let parent_height = parent_bounds.size.height;

    let child_x = parent_x + (parent_width - px(w)) / 2.0;
    let child_y = parent_y + (parent_height - px(h)) / 2.0;
    let mut window_options = WindowOptions::default();
    let window_size = size(px(w), px(h));

    let bounds = Bounds {
        origin: Point { x: child_x, y:child_y },
        size:window_size,
    };
    window_options.window_bounds = Some(WindowBounds::Windowed(bounds));
    window_options.window_min_size = Some(window_size);
    window_options
}