use anyhow::Result;
use gpui::*;

use crate::{
    domain::session::{NewSession, SessionProfile},
    global_state::{GlobalEvent, read_global_state},
    infrastructure::storage::Storage,
};

use super::{SessionFormMode, SessionOperationWindow};

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

pub(crate) fn open_new_session_window(cx: &mut App) {
    open_session_window(None, cx);
}

pub(crate) fn open_edit_session_window<T: 'static>(
    profile: SessionProfile,
    _session_list: Entity<T>,
    cx: &mut App,
) {
    open_session_window(Some(profile), cx);
}

fn open_session_window(profile: Option<SessionProfile>, cx: &mut App) {
    let editing = profile.is_some();
    let window_size = size(px(680.), px(500.));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::centered(window_size, cx)),
        window_min_size: Some(window_size),
        titlebar: Some(TitlebarOptions {
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
        }),
        kind: WindowKind::Dialog,
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };

    let _ = cx.open_window(options, move |window, cx| {
        let form = match profile {
            Some(profile) => cx.new(|cx| SessionOperationWindow::edit(profile, window, cx)),
            None => cx.new(|cx| SessionOperationWindow::new(window, cx)),
        };
        cx.new(|cx| gpui_component::Root::new(form, window, cx))
    });
}
