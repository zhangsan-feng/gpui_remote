use anyhow::Result;
use gpui_kit::*;

use super::{SessionFormMode, SessionOperationWindow};
use crate::component::window::window_center_options;
use crate::{
    application::ApplicationContext,
    domain::session::{NewSession, SessionProfile},
    global_state::GlobalEvent,
};

impl SessionOperationWindow {
    pub(super) async fn save_session(
        mode: SessionFormMode,
        draft: NewSession,
        application: ApplicationContext,
    ) -> Result<GlobalEvent> {
        let result = match &mode {
            SessionFormMode::Create => application.create_session(draft).await,
            SessionFormMode::Edit { id } => application.update_session(id.clone(), draft).await,
        };
        result
            .map(|_| match mode {
                SessionFormMode::Create => GlobalEvent::CreateSession,
                SessionFormMode::Edit { .. } => GlobalEvent::UpdateSession,
            })
            .map_err(anyhow::Error::msg)
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
    let mut options = window_center_options(window, size(px(680.), px(500.)));
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
        window.on_window_should_close(cx, |window, _| {
            window.remove_window();
            false
        });
        let form = match profile {
            Some(profile) => cx.new(|cx| SessionOperationWindow::edit(profile, window, cx)),
            None => cx.new(|cx| SessionOperationWindow::new(window, cx)),
        };
        cx.new(|cx| gpui_kit::component::Root::new(form, window, cx))
    });
}
