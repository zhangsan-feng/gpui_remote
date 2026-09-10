use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use crate::domain::session::Protocol;

use super::{
    DataContextCommand, DataContextResult, SftpCommand, SftpDirectorySummary, SftpTransferInfo,
    SftpTransferSummary, SftpWatchSummary, SshCommand, TerminalReadPage, TerminalSummary,
};

use super::bus::{GUI_COMMAND_QUEUE_CAPACITY, GUI_REQUEST_TIMEOUT};

#[derive(Clone)]
pub struct GuiContext {
    commands: mpsc::Sender<DataContextCommand>,
}

pub struct GuiContextReceiver {
    commands: mpsc::Receiver<DataContextCommand>,
}

pub fn gui_context_channel() -> (GuiContext, GuiContextReceiver) {
    let (commands, receiver) = mpsc::channel(GUI_COMMAND_QUEUE_CAPACITY);
    (
        GuiContext { commands },
        GuiContextReceiver { commands: receiver },
    )
}

impl GuiContextReceiver {
    pub async fn recv(&mut self) -> Option<DataContextCommand> {
        self.commands.recv().await
    }
}

impl GuiContext {
    pub async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    ) -> DataContextResult<String> {
        self.request(|reply| match protocol {
            Protocol::Ssh => DataContextCommand::Ssh(SshCommand::Open {
                profile_id,
                ip,
                title,
                reply,
            }),
            Protocol::Sftp => DataContextCommand::Sftp(SftpCommand::Open {
                profile_id,
                ip,
                title,
                reply,
            }),
        })
        .await
    }

    pub async fn list_sftp_local(&self) -> DataContextResult<SftpDirectorySummary> {
        self.request(|reply| DataContextCommand::Sftp(SftpCommand::ListLocal { reply }))
            .await
    }

    pub async fn list_sftp_sessions(&self) -> DataContextResult<Vec<TerminalSummary>> {
        self.request(|reply| DataContextCommand::Sftp(SftpCommand::ListSessions { reply }))
            .await
    }

    pub async fn change_sftp_local_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> DataContextResult<()> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::ChangeLocalDirectory {
                workspace_id,
                ip,
                title,
                path,
                reply,
            })
        })
        .await
    }

    pub async fn list_sftp_remote(
        &self,
        workspace_id: String,
    ) -> DataContextResult<SftpDirectorySummary> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::ListRemote {
                workspace_id,
                reply,
            })
        })
        .await
    }

    pub async fn change_sftp_remote_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> DataContextResult<()> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::ChangeRemoteDirectory {
                workspace_id,
                ip,
                title,
                path,
                reply,
            })
        })
        .await
    }

    pub async fn upload_sftp(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> DataContextResult<SftpTransferSummary> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::Upload {
                workspace_id,
                local_paths,
                reply,
            })
        })
        .await
    }

    pub async fn download_sftp(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> DataContextResult<SftpTransferSummary> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::Download {
                workspace_id,
                remote_paths,
                reply,
            })
        })
        .await
    }

    pub async fn list_sftp_transfers(
        &self,
        workspace_id: String,
    ) -> DataContextResult<Vec<SftpTransferInfo>> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::ListTransfers {
                workspace_id,
                reply,
            })
        })
        .await
    }

    pub async fn watch_sftp_local(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> DataContextResult<SftpWatchSummary> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::WatchLocal {
                workspace_id,
                ip,
                title,
                local_path,
                reply,
            })
        })
        .await
    }

    pub async fn stop_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> DataContextResult<()> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::StopWatchingLocal {
                workspace_id,
                ip,
                title,
                local_path,
                reply,
            })
        })
        .await
    }

    pub async fn list_sftp_local_watches(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> DataContextResult<Vec<SftpWatchSummary>> {
        self.request(|reply| {
            DataContextCommand::Sftp(SftpCommand::ListLocalWatches {
                workspace_id,
                ip,
                title,
                reply,
            })
        })
        .await
    }

    pub async fn list_terminals(&self) -> DataContextResult<Vec<TerminalSummary>> {
        self.request(|reply| DataContextCommand::Ssh(SshCommand::ListTerminals { reply }))
            .await
    }

    pub async fn select_terminal(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> DataContextResult<()> {
        self.request(|reply| {
            DataContextCommand::Ssh(SshCommand::SelectTerminal {
                workspace_id,
                ip,
                title,
                reply,
            })
        })
        .await
    }

    pub async fn read_terminal(
        &self,
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    ) -> DataContextResult<TerminalReadPage> {
        self.request(|reply| {
            DataContextCommand::Ssh(SshCommand::ReadTerminal {
                workspace_id,
                offset,
                limit,
                reply,
            })
        })
        .await
    }

    pub async fn send_text(
        &self,
        workspace_id: Option<String>,
        text: String,
    ) -> DataContextResult<()> {
        self.request(|reply| {
            DataContextCommand::Ssh(SshCommand::SendText {
                workspace_id,
                text,
                reply,
            })
        })
        .await
    }

    pub async fn send_key(
        &self,
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> DataContextResult<()> {
        self.request(|reply| {
            DataContextCommand::Ssh(SshCommand::SendKey {
                workspace_id,
                key,
                control,
                alt,
                shift,
                reply,
            })
        })
        .await
    }

    async fn request<T>(
        &self,
        command: impl FnOnce(oneshot::Sender<DataContextResult<T>>) -> DataContextCommand,
    ) -> DataContextResult<T> {
        self.request_with_timeout(command, GUI_REQUEST_TIMEOUT)
            .await
    }

    async fn request_with_timeout<T>(
        &self,
        command: impl FnOnce(oneshot::Sender<DataContextResult<T>>) -> DataContextCommand,
        timeout: Duration,
    ) -> DataContextResult<T> {
        let (reply, response) = oneshot::channel();
        let queue_capacity = self.commands.capacity();
        let deadline = tokio::time::Instant::now() + timeout;
        let command = command(reply);
        let command_name = command.name();

        if queue_capacity == 0 {
            log::warn!(
                "DataContext GUI command queue is full; command={command_name}, waiting for capacity"
            );
        }

        tokio::time::timeout_at(deadline, self.commands.send(command))
            .await
            .map_err(|_| {
                log::warn!(
                    "DataContext GUI command queue timed out after {:?}; command={command_name}, capacity_before_send={queue_capacity}",
                    timeout
                );
                "DataContext GUI command queue timed out".to_owned()
            })?
            .map_err(|_| {
                log::warn!("DataContext GUI command rejected: command={command_name}");
                "DataContext GUI bridge is unavailable".to_owned()
            })?;

        log::debug!(
            "DataContext GUI command dispatched: command={command_name}, capacity_after_send={}",
            self.commands.capacity()
        );

        let response = tokio::time::timeout_at(deadline, response)
            .await
            .map_err(|_| {
                log::warn!(
                    "DataContext GUI response timed out after {:?}: command={command_name}",
                    timeout
                );
                "DataContext GUI request timed out".to_owned()
            })?
            .map_err(|_| "DataContext GUI request was cancelled".to_owned())?;

        log::debug!("DataContext GUI response received: command={command_name}");
        response
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::sync::oneshot;

    use super::super::bus::GUI_COMMAND_QUEUE_CAPACITY;
    use super::gui_context_channel;

    #[test]
    fn gui_context_queue_keeps_the_bounded_capacity() {
        let (context, _receiver) = gui_context_channel();

        assert_eq!(context.commands.capacity(), GUI_COMMAND_QUEUE_CAPACITY);
    }

    #[tokio::test]
    async fn gui_context_routes_commands_to_the_receiver() {
        let (context, mut receiver) = gui_context_channel();
        let request = tokio::spawn(async move { context.list_terminals().await });

        let command = receiver.recv().await.expect("GUI command expected");
        assert_eq!(command.name(), "ssh.list_terminals");

        if let super::DataContextCommand::Ssh(super::SshCommand::ListTerminals { reply }) = command
        {
            reply
                .send(Ok(Vec::new()))
                .expect("request should still wait");
        } else {
            panic!("expected SSH terminal command");
        }

        assert!(request.await.expect("request task should finish").is_ok());
    }

    #[tokio::test]
    async fn request_returns_an_error_when_the_gui_queue_is_full() {
        let (context, _receiver) = gui_context_channel();

        for _ in 0..GUI_COMMAND_QUEUE_CAPACITY {
            let (reply, _response) = oneshot::channel();
            context
                .commands
                .try_send(super::DataContextCommand::Ssh(
                    super::SshCommand::ListTerminals { reply },
                ))
                .expect("the test queue should have capacity");
        }

        let result = context
            .request_with_timeout(
                |reply| super::DataContextCommand::Ssh(super::SshCommand::ListTerminals { reply }),
                Duration::from_millis(20),
            )
            .await;

        assert!(matches!(
            result,
            Err(error) if error == "DataContext GUI command queue timed out"
        ));
    }
}
