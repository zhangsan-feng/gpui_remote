use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::ui::render_about_dialog;

pub fn open_about_dialog(window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, render_about_dialog);
}
