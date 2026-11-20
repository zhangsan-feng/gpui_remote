use gpui_kit::*;

use crate::{
    application::{APPLICATION, run_application},
    domain::session::SessionGroupTarget,
};

use super::MoveSessionWindow;

impl MoveSessionWindow {
    pub(super) fn move_existing(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.submit(
            SessionGroupTarget::Existing(self.selected.clone()),
            window,
            cx,
        );
    }

    pub(super) fn create_and_move(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = self.name.read(cx).value().trim().to_owned();
        if name.is_empty() {
            return;
        }
        self.submit(
            SessionGroupTarget::New {
                parent_id: self.selected.clone(),
                name,
            },
            window,
            cx,
        );
    }

    fn submit(&mut self, target: SessionGroupTarget, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let handle = window.window_handle();
        let on_moved = self.on_moved.clone();
        let selected = self.selected.clone();
        let session_id = self.session.id.clone();
        let application = APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = run_application(async move {
                application
                    .sessions
                    .move_session_to_group(session_id, target)
                    .await
            })
            .await;
            match result {
                Ok(group_id) => {
                    let _ = cx.update(|cx| on_moved(cx, selected.as_deref(), group_id.as_deref()));
                    let _ = cx.update_window(handle, |_, window, _| window.remove_window());
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.busy = false;
                        this.error = Some(error);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
        cx.notify();
    }
}
