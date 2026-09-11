use anyhow::Error;
use gpui_kit::{AppContext, Context, Window};

use crate::{
    domain::session::Protocol, gui::title_bar::session_operation_window::open_edit_session_window,
};

use super::{ConnectSession, ConnectSftpSession, DeleteSession, EditSession, SessionComponent};

impl SessionComponent {
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
        match self.find_session_in_projection(session_id) {
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
        match self.find_session_in_projection(&action.0) {
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
        let application =
            cx.read_global::<crate::application::ApplicationContext, _>(|application, _| {
                application.clone()
            });
        let session_id = action.0.clone();
        cx.spawn(async move |this, cx| {
            let result = application.delete_session(session_id).await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(()) => this.refresh_sessions(cx),
                Err(error) => this.set_error(Error::msg(error), cx),
            });
        })
        .detach();
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
        match self.find_session_in_projection(session_id) {
            Ok(mut profile) => {
                profile.protocol = protocol;
                self.open_workspace(profile, cx);
            }
            Err(error) => self.set_error(error, cx),
        }
    }
}
