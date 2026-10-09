use anyhow::Error;
use gpui_kit::{AppContext, Context, Window};

use crate::{
    domain::session::Protocol, gui::title_bar::session_operation_window::open_edit_session_window,
};

use super::{
    CloseSshTunnel, ConnectSession, ConnectSftpSession, DeleteSession, EditSession, OpenSshTunnel,
    SessionComponent,
};

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

    pub(super) fn open_ssh_tunnel(
        &mut self,
        action: &OpenSshTunnel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_ssh_tunnel_state(&action.0, true, cx);
    }

    pub(super) fn close_ssh_tunnel(
        &mut self,
        action: &CloseSshTunnel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_ssh_tunnel_state(&action.0, false, cx);
    }

    pub(super) fn create_active_session_by_id(&mut self, session_id: &str, cx: &mut Context<Self>) {
        match self.find_session_in_projection(session_id) {
            Ok(session) => self.open_workspace(session, Protocol::Ssh, cx),
            Err(error) => self.set_error(error, cx),
        }
    }

    pub(super) fn edit_session(
        &mut self,
        action: &EditSession,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.find_session_in_projection(&action.0) {
            self.set_error(error, cx);
            return;
        }
        let application = crate::application::APPLICATION.clone();
        let session_id = action.0.clone();
        let window_handle = window.window_handle();
        let session_list = cx.entity();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.sessions.find_profile(session_id).await
            })
            .await;
            match result {
                Ok(profile) => {
                    let _ = cx.update_window(window_handle, |_, window, cx| {
                        open_edit_session_window(profile, session_list, window, cx);
                    });
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.set_error(Error::msg(error), cx);
                    });
                }
            }
        })
        .detach();
    }

    pub(super) fn delete_session(
        &mut self,
        action: &DeleteSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let application = crate::application::APPLICATION.clone();
        let session_id = action.0.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.sessions.delete_profile(session_id).await
            })
            .await;
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
            Ok(session) => self.open_workspace(session, protocol, cx),
            Err(error) => self.set_error(error, cx),
        }
    }

    fn set_ssh_tunnel_state(&mut self, session_id: &str, open: bool, cx: &mut Context<Self>) {
        let session = match self.find_session_in_projection(session_id) {
            Ok(session) => session,
            Err(error) => {
                self.set_error(error, cx);
                return;
            }
        };
        let application = crate::application::APPLICATION.clone();
        let session_id = session_id.to_owned();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                if open {
                    application.ssh.open_reverse_tunnel(session_id).await
                } else {
                    application.ssh.close_reverse_tunnel(session_id).await
                }
            })
            .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(()) => {
                    this.refresh_sessions(cx);
                    if open {
                        this.open_workspace_with_tunnel_proxy(session, Protocol::Ssh, true, cx);
                    }
                }
                Err(error) => this.set_error(Error::msg(error), cx),
            });
        })
        .detach();
    }
}
