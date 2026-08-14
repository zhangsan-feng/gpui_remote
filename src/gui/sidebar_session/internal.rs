use anyhow::{Error, Result};
use gpui::{Context, Window};

use crate::{
    domain::session::Protocol, gui::title_bar::session_operation_window::open_edit_session_window,
};

use super::{ConnectSession, ConnectSftpSession, DeleteSession, EditSession, SessionComponent};

impl SessionComponent {
    pub(super) fn reload_session(&mut self, cx: &mut Context<Self>) -> Result<()> {
        self.sessions = self.load_sessions(cx)?;
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
        self.open_session_with_protocol(&action.0, Protocol::Ssh, cx);
    }

    pub(super) fn create_active_sftp_session(
        &mut self,
        action: &ConnectSftpSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_session_with_protocol(&action.0, Protocol::Sftp, cx);
    }

    pub(super) fn create_active_session_by_id(&mut self, session_id: &str, cx: &mut Context<Self>) {
        match self.query_session(session_id, cx) {
            Ok(profile) => self.open_workspace(profile, cx),
            Err(error) => self.set_error(error, cx),
        }
    }

    pub(super) fn edit_session(
        &mut self,
        action: &EditSession,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.query_session(&action.0, cx) {
            Ok(profile) => open_edit_session_window(profile, cx.entity(), window, cx),
            Err(error) => self.set_error(error, cx),
        }
    }

    pub(super) fn delete_session(
        &mut self,
        action: &DeleteSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self
            .remove_session(&action.0, cx)
            .and_then(|()| self.reload_session(cx))
        {
            self.set_error(error, cx);
        }
    }

    pub(super) fn set_error(&mut self, error: Error, cx: &mut Context<Self>) {
        self.core_err = Some(error);
        cx.notify();
    }

    fn open_session_with_protocol(
        &mut self,
        session_id: &str,
        protocol: Protocol,
        cx: &mut Context<Self>,
    ) {
        match self.query_session(session_id, cx) {
            Ok(mut profile) => {
                profile.protocol = protocol;
                self.open_workspace(profile, cx);
            }
            Err(error) => self.set_error(error, cx),
        }
    }
}
