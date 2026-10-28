#[cfg(not(windows))]
use std::rc::Rc;

use gpui_kit::App;
#[cfg(not(windows))]
use gpui_kit::{ClipboardItem, Global, Platform};

#[cfg(not(windows))]
struct ClipboardPlatform(Rc<dyn Platform>);

#[cfg(not(windows))]
impl Global for ClipboardPlatform {}

#[cfg(not(windows))]
pub(crate) fn init(platform: Rc<dyn Platform>, cx: &mut App) {
    cx.set_global(ClipboardPlatform(platform));
}

pub(crate) fn write_text(text: String, cx: &mut App) {
    #[cfg(windows)]
    {
        let bytes = text.len();
        cx.spawn(async move |_| {
            let result = tokio::spawn(async move {
                crate::application::APPLICATION
                    .write_clipboard_text(text)
                    .await
            })
            .await;
            match result {
                Ok(Ok(())) => log::info!("terminal_clipboard_copy_completed bytes={bytes}"),
                Ok(Err(error)) => {
                    log::warn!("terminal_clipboard_copy_failed bytes={bytes}: {error}")
                }
                Err(error) => log::warn!("terminal_clipboard_copy_task_failed: {error}"),
            }
        })
        .detach();
    }
    #[cfg(not(windows))]
    {
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
}
