use std::rc::Rc;

use gpui_kit::{App, ClipboardItem, Global, Platform};

struct ClipboardPlatform(Rc<dyn Platform>);

impl Global for ClipboardPlatform {}

pub(crate) fn init(platform: Rc<dyn Platform>, cx: &mut App) {
    cx.set_global(ClipboardPlatform(platform));
}

pub(crate) fn write_text(text: String, cx: &mut App) {
    let platform = cx.global::<ClipboardPlatform>().0.clone();
    let bytes = text.len();
    // Windows clipboard calls can dispatch window messages synchronously.
    // Run after the event handler returns, without borrowing App through cx.update.
    cx.spawn(async move |_| {
        log::debug!("Clipboard write started: bytes={bytes}");
        platform.write_to_clipboard(ClipboardItem::new_string(text));
        log::debug!("Clipboard write returned: bytes={bytes}");
    })
    .detach();
}
