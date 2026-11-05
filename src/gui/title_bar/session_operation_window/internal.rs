use gpui_kit::*;

use super::{FormSection, SessionOperationWindow};
use crate::event_bus::EVENT_BUS;

impl SessionOperationWindow {
    pub(super) fn submit(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let draft = match self.draft(cx) {
            Ok(draft) => draft,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };

        let application = crate::application::APPLICATION.clone();
        let mode = self.mode.clone();
        let window_handle = window.window_handle();
        cx.spawn(async move |this, cx| {
            match SessionOperationWindow::save_session(mode, draft, application).await {
                Ok(event) => {
                    EVENT_BUS.gui_event.broadcast(event);
                    let _ = cx.update_window(window_handle, |_, window, _| window.remove_window());
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.error = Some(error.to_string());
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub(super) fn cancel(&mut self, _: &ClickEvent, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    pub(super) fn select_section(&mut self, section: FormSection, cx: &mut Context<Self>) {
        self.section = section;
        self.error = None;
        cx.notify();
    }
}
