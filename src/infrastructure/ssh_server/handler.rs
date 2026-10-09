use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use alacritty_terminal::event::WindowSize;
use russh::{
    Channel, ChannelId, Pty,
    server::{self, Msg, Session},
};
use tokio::sync::mpsc;

use super::shell::{ShellCommand, spawn_shell, window_size};
use crate::{
    application::sftp_server::SftpServerApplication, infrastructure::sftp_server::LocalSftpHandler,
};

pub(super) struct SshServer {
    username: Arc<str>,
    password: Arc<str>,
    sftp_root: Arc<PathBuf>,
    sftp_application: Arc<SftpServerApplication>,
}

impl SshServer {
    pub(super) fn new(
        username: String,
        password: String,
        sftp_root: PathBuf,
        sftp_application: Arc<SftpServerApplication>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            sftp_root: Arc::new(sftp_root),
            sftp_application,
        }
    }
}

struct ChannelState {
    channel: Option<Channel<Msg>>,
    size: Option<WindowSize>,
    commands: Option<mpsc::Sender<ShellCommand>>,
}

pub(super) struct SshClientHandler {
    username: Arc<str>,
    password: Arc<str>,
    sftp_root: Arc<PathBuf>,
    sftp_application: Arc<SftpServerApplication>,
    channels: HashMap<ChannelId, ChannelState>,
}

impl server::Server for SshServer {
    type Handler = SshClientHandler;

    fn new_client(&mut self, _peer_addr: Option<SocketAddr>) -> Self::Handler {
        SshClientHandler {
            username: self.username.clone(),
            password: self.password.clone(),
            sftp_root: self.sftp_root.clone(),
            sftp_application: self.sftp_application.clone(),
            channels: HashMap::new(),
        }
    }

    fn handle_session_error(&mut self, error: russh::Error) {
        log::warn!("ssh_server_session_error: {error}");
    }
}

impl server::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> Result<server::Auth, Self::Error> {
        let accepted = user == &*self.username && password == &*self.password;
        log::info!("ssh_server_password_auth user={user} accepted={accepted}");
        Ok(if accepted {
            server::Auth::Accept
        } else {
            server::Auth::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(
            channel.id(),
            ChannelState {
                channel: Some(channel),
                size: None,
                commands: None,
            },
        );
        reply.accept().await;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name != "sftp" {
            log::warn!("ssh_server_subsystem_rejected channel={channel:?} name={name}");
            session.channel_failure(channel)?;
            return Ok(());
        }

        let Some(mut state) = self.channels.remove(&channel) else {
            session.channel_failure(channel)?;
            return Ok(());
        };
        let Some(channel_stream) = state.channel.take() else {
            session.channel_failure(channel)?;
            return Ok(());
        };
        if state.commands.is_some() {
            session.channel_failure(channel)?;
            return Ok(());
        }

        session.channel_success(channel)?;
        log::info!("ssh_server_sftp_started channel={channel:?}");
        russh_sftp::server::run(
            channel_stream.into_stream(),
            LocalSftpHandler::new(
                self.sftp_application.clone(),
                self.sftp_root.as_ref().clone(),
            ),
        )
        .await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        columns: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
        _modes: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(state) = self.channels.get_mut(&channel) {
            state.size = Some(window_size(columns, rows, pixel_width, pixel_height));
            session.channel_success(channel)?;
        } else {
            session.channel_failure(channel)?;
        }
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let size = self.channels.get(&channel).and_then(|state| state.size);
        if let Some(size) = size {
            match spawn_shell(size, session.handle(), channel).await {
                Ok(commands) => {
                    if let Some(state) = self.channels.get_mut(&channel) {
                        state.commands = Some(commands);
                    }
                    session.channel_success(channel)?;
                }
                Err(error) => {
                    log::error!("ssh_server_shell_failed: {error}");
                    session.channel_failure(channel)?;
                }
            }
        } else {
            session.channel_failure(channel)?;
        }
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        columns: u32,
        rows: u32,
        pixel_width: u32,
        pixel_height: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let size = window_size(columns, rows, pixel_width, pixel_height);
        if let Some(state) = self.channels.get_mut(&channel) {
            state.size = Some(size);
            if let Some(commands) = &state.commands {
                let _ = commands.send(ShellCommand::Resize(size)).await;
            }
            session.channel_success(channel)?;
        } else {
            session.channel_failure(channel)?;
        }
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(commands) = self
            .channels
            .get(&channel)
            .and_then(|state| state.commands.as_ref())
        {
            if commands
                .send(ShellCommand::Input(data.to_vec()))
                .await
                .is_err()
            {
                log::warn!("ssh_server_shell_input_closed");
            }
        }
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(state) = self.channels.remove(&channel) {
            if let Some(commands) = state.commands {
                let _ = commands.send(ShellCommand::Stop).await;
            }
        }
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channel_close(channel, session).await
    }
}

impl Drop for SshClientHandler {
    fn drop(&mut self) {
        for state in self.channels.values() {
            if let Some(commands) = &state.commands {
                let _ = commands.try_send(ShellCommand::Stop);
            }
        }
    }
}
