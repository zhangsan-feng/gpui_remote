use crate::{
    domain::session::SessionProfile,
    global_state::{GlobalEvent, read_global_state},
};
use gpui_kit::Context;

use super::SessionComponent;

impl SessionComponent {
    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_events = read_global_state(cx);
        cx.subscribe(&global_events, |this, _, event, cx| {
            match event {
                GlobalEvent::CreateSession | GlobalEvent::UpdateSession => {}
                GlobalEvent::ThemeColorChanged => {
                    this.refer_item(cx);
                    cx.notify();
                    return;
                }
                _ => return,
            }
            if let Err(error) = this.reload_session(cx) {
                this.set_error(error, cx);
            }
        })
        .detach();
    }

    pub(super) fn open_workspace(&self, profile: SessionProfile, cx: &mut Context<Self>) {
        read_global_state(cx).update(cx, |_, cx| {
            cx.emit(GlobalEvent::OpenWorkspaceSession(profile));
        });
    }
}
