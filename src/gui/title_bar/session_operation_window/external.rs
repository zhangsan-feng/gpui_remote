use anyhow::Result;
use gpui::*;

use super::{SessionFormMode, SessionOperationWindow};
use crate::component::window::window_center_options;
use crate::{
    domain::session::{NewSession, SessionProfile},
    global_state::{GlobalEvent, read_global_state},
    infrastructure::storage::Storage,
};

impl SessionOperationWindow {
    pub(super) fn persist(&self, draft: NewSession, cx: &App) -> Result<GlobalEvent> {
        match &self.mode {
            SessionFormMode::Create => cx
                .global::<Storage>()
                .session
                .insert(draft)
                .map(|_| GlobalEvent::CreateSession),
            SessionFormMode::Edit { id } => cx
                .global::<Storage>()
                .session
                .update(id, draft)
                .map(|_| GlobalEvent::UpdateSession),
        }
    }

    pub(super) fn publish_change(&self, event: GlobalEvent, cx: &mut Context<Self>) {
        read_global_state(cx).update(cx, |_, cx| {
            cx.emit(event);
        });
    }
}

pub(crate) fn open_new_session_window(window: &mut Window, cx: &mut App) {
    open_session_window(None, window, cx);
}

pub(crate) fn open_edit_session_window<T: 'static>(
    profile: SessionProfile,
    _session_list: Entity<T>,
    window: &mut Window,
    cx: &mut App,
) {
    open_session_window(Some(profile), window, cx);
}

fn open_session_window(profile: Option<SessionProfile>, window: &mut Window, cx: &mut App) {
    let editing = profile.is_some();
    let mut options = window_center_options(window, 680., 500.);
    options.titlebar = Some(TitlebarOptions {
        title: Some(
            if editing {
                "编辑远程会话"
            } else {
                "新建远程会话"
            }
            .into(),
        ),
        appears_transparent: false,
        traffic_light_position: None,
    });
    options.kind = WindowKind::Dialog;
    options.is_resizable = false;
    options.is_minimizable = false;

    let _ = cx.open_window(options, move |window, cx| {
        let form = match profile {
            Some(profile) => cx.new(|cx| SessionOperationWindow::edit(profile, window, cx)),
            None => cx.new(|cx| SessionOperationWindow::new(window, cx)),
        };
        cx.new(|cx| gpui_component::Root::new(form, window, cx))
    });
}
