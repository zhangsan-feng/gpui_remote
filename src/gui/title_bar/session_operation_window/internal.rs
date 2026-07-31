use gpui::*;

use super::{ConnectionProtocol, FormSection, SessionOperationWindow};

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

        match self.persist(draft, cx) {
            Ok(event) => {
                self.publish_change(event, cx);
                window.remove_window();
            }
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
            }
        }
    }

    pub(super) fn cancel(&mut self, _: &ClickEvent, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }

    pub(super) fn select_protocol(&mut self, protocol: ConnectionProtocol, cx: &mut Context<Self>) {
        self.protocol = protocol;
        self.error = None;
        cx.notify();
    }

    pub(super) fn select_section(&mut self, section: FormSection, cx: &mut Context<Self>) {
        self.section = section;
        self.error = None;
        cx.notify();
    }
}
