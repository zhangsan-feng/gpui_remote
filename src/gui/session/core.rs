use anyhow::{Context as _, Result};
use gpui::{Context, Window};

use crate::{
    domain::session::SessionProfile,
    global_state::{GlobalEvent, read_global_state},
    gui::{
        session::{ConnectSession, DeleteSession, EditSession, SessionComponent},
        title_bar::session_operation_window::open_edit_session_window,
    },
    infrastructure::storage::Storage,
};

impl SessionComponent {
    pub fn query_session(&self, id: &str, cx: &Context<Self>) -> Result<SessionProfile> {
        cx.global::<Storage>()
            .session
            .list()?
            .into_iter()
            .find(|session| session.id == id)
            .with_context(|| format!("session not found: {id}"))
    }

    pub fn reload_session(&mut self, cx: &mut Context<Self>) -> Result<()> {
        self.sessions = cx.global::<Storage>().session.list()?;
        self.core_err = None;
        self.render_item(cx);
        self.refer_item(cx);
        cx.notify();
        Ok(())
    }

    pub(super) fn create_active_session(
        &mut self,
        action: &ConnectSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.create_active_session_by_id(&action.0, cx);
    }

    pub(super) fn create_active_session_by_id(&mut self, session_id: &str, cx: &mut Context<Self>) {
        match self.query_session(session_id, cx) {
            Ok(profile) => {
                read_global_state(cx).update(cx, |_, cx| {
                    cx.emit(GlobalEvent::CreateActiveSession(profile));
                });
            }
            Err(error) => {
                self.core_err = Some(error);
                cx.notify();
            }
        }
    }

    pub(super) fn edit_session(
        &mut self,
        action: &EditSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.query_session(&action.0, cx) {
            Ok(profile) => open_edit_session_window(profile, cx.entity(), cx),
            Err(error) => {
                self.core_err = Some(error);
                cx.notify();
            }
        }
    }

    pub(super) fn delete_session(
        &mut self,
        action: &DeleteSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = cx.global::<Storage>().session.delete(&action.0);
        match result {
            Ok(()) => {
                let profile_id = action.0.clone();
                read_global_state(cx).update(cx, |_, cx| {
                    cx.emit(GlobalEvent::SessionProfileDeleted(profile_id));
                });
                if let Err(error) = self.reload_session(cx) {
                    self.core_err = Some(error);
                    cx.notify();
                }
            }
            Err(error) => {
                self.core_err = Some(error);
                cx.notify();
            }
        }
    }
}
