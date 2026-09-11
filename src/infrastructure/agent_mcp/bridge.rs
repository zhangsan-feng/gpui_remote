use gpui_kit::{App, AppContext};
use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use crate::{
    application::{
        ApplicationContext, ApplicationEvent, ApplicationResult,
        model::{
            ProfileSummary, SftpDirectorySummary, SftpTransferInfo, SftpTransferSummary,
            SftpWatchSummary, TerminalReadPage, TerminalSummary,
        },
    },
    domain::session::Protocol,
};

const COMMAND_CAPACITY: usize = 128;
const NOTIFICATION_CAPACITY: usize = 256;

pub(crate) enum ApplicationCommand {
    ListProfiles,
    OpenSession {
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    },
    CloseSession {
        workspace_id: String,
    },
    ListSftpSessions,
    ListSftpLocal,
    ChangeSftpLocalDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    },
    ListSftpRemote {
        workspace_id: String,
    },
    ChangeSftpRemoteDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    },
    UploadSftp {
        workspace_id: String,
        local_paths: Vec<String>,
    },
    DownloadSftp {
        workspace_id: String,
        remote_paths: Vec<String>,
    },
    ListSftpTransfers {
        workspace_id: String,
    },
    WatchSftpLocal {
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    },
    StopSftpLocalWatch {
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    },
    ListSftpLocalWatches {
        workspace_id: String,
        ip: String,
        title: String,
    },
    ListTerminals,
    SelectTerminal {
        workspace_id: String,
        ip: String,
        title: String,
    },
    ReadTerminal {
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    },
    SendText {
        workspace_id: Option<String>,
        text: String,
    },
    SendKey {
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    },
}

pub(crate) enum ApplicationResponse {
    Empty,
    Profiles(Vec<ProfileSummary>),
    WorkspaceId(String),
    TerminalSummaries(Vec<TerminalSummary>),
    TerminalRead(TerminalReadPage),
    SftpDirectory(SftpDirectorySummary),
    SftpTransferSummary(SftpTransferSummary),
    SftpTransferInfos(Vec<SftpTransferInfo>),
    SftpWatch(SftpWatchSummary),
    SftpWatches(Vec<SftpWatchSummary>),
}

#[derive(Clone, Debug)]
pub(crate) enum ApplicationNotification {
    SessionOpened {
        workspace_id: String,
        profile: ProfileSummary,
    },
    SessionClosed {
        workspace_id: String,
    },
    SessionSelected {
        workspace_id: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct NotificationEnvelope {
    pub(crate) event: ApplicationNotification,
}

pub(crate) struct CommandEnvelope {
    pub(crate) request_id: String,
    pub(crate) command: ApplicationCommand,
    pub(crate) response_tx: oneshot::Sender<ResponseEnvelope>,
}

pub(crate) struct ResponseEnvelope {
    pub(crate) request_id: String,
    pub(crate) result: ApplicationResult<ApplicationResponse>,
}

#[derive(Clone)]
pub(crate) struct McpBridgeEndpoint {
    command_tx: mpsc::Sender<CommandEnvelope>,
    notification_tx: broadcast::Sender<NotificationEnvelope>,
}

pub(crate) struct McpBridgeReceiver {
    pub(crate) command_rx: mpsc::Receiver<CommandEnvelope>,
    pub(crate) notification_tx: broadcast::Sender<NotificationEnvelope>,
}

pub(crate) fn new() -> (McpBridgeEndpoint, McpBridgeReceiver) {
    let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
    let (notification_tx, _) = broadcast::channel(NOTIFICATION_CAPACITY);
    (
        McpBridgeEndpoint {
            command_tx,
            notification_tx: notification_tx.clone(),
        },
        McpBridgeReceiver {
            command_rx,
            notification_tx,
        },
    )
}

impl McpBridgeEndpoint {
    pub(crate) async fn request(
        &self,
        command: ApplicationCommand,
    ) -> ApplicationResult<ApplicationResponse> {
        let request_id = Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();
        self.command_tx
            .send(CommandEnvelope {
                request_id: request_id.clone(),
                command,
                response_tx,
            })
            .await
            .map_err(|_| "MCP application bridge 已关闭".to_owned())?;
        let response = response_rx
            .await
            .map_err(|_| "MCP application bridge 未返回响应".to_owned())?;
        if response.request_id != request_id {
            return Err(format!(
                "MCP application bridge 响应关联 ID 不匹配: expected={request_id}, actual={}",
                response.request_id
            ));
        }
        response.result
    }

    pub(crate) fn subscribe_notifications(&self) -> broadcast::Receiver<NotificationEnvelope> {
        self.notification_tx.subscribe()
    }

    pub(crate) async fn list_profiles(&self) -> ApplicationResult<Vec<ProfileSummary>> {
        match self.request(ApplicationCommand::ListProfiles).await? {
            ApplicationResponse::Profiles(profiles) => Ok(profiles),
            _ => Err("MCP application bridge 返回了错误的 profiles 响应".to_owned()),
        }
    }

    pub(crate) async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    ) -> ApplicationResult<String> {
        match self
            .request(ApplicationCommand::OpenSession {
                profile_id,
                protocol,
                ip,
                title,
            })
            .await?
        {
            ApplicationResponse::WorkspaceId(workspace_id) => Ok(workspace_id),
            _ => Err("MCP application bridge 返回了错误的 open_session 响应".to_owned()),
        }
    }

    pub(crate) async fn list_sftp_sessions(&self) -> ApplicationResult<Vec<TerminalSummary>> {
        match self.request(ApplicationCommand::ListSftpSessions).await? {
            ApplicationResponse::TerminalSummaries(sessions) => Ok(sessions),
            _ => Err("MCP application bridge 返回了错误的 SFTP 会话响应".to_owned()),
        }
    }

    pub(crate) async fn close_session(&self, workspace_id: String) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::CloseSession { workspace_id })
            .await
    }

    pub(crate) async fn list_sftp_local(&self) -> ApplicationResult<SftpDirectorySummary> {
        match self.request(ApplicationCommand::ListSftpLocal).await? {
            ApplicationResponse::SftpDirectory(directory) => Ok(directory),
            _ => Err("MCP application bridge 返回了错误的本地目录响应".to_owned()),
        }
    }

    pub(crate) async fn change_sftp_local_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::ChangeSftpLocalDirectory {
            workspace_id,
            ip,
            title,
            path,
        })
        .await
    }

    pub(crate) async fn list_sftp_remote(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<SftpDirectorySummary> {
        match self
            .request(ApplicationCommand::ListSftpRemote { workspace_id })
            .await?
        {
            ApplicationResponse::SftpDirectory(directory) => Ok(directory),
            _ => Err("MCP application bridge 返回了错误的远程目录响应".to_owned()),
        }
    }

    pub(crate) async fn change_sftp_remote_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::ChangeSftpRemoteDirectory {
            workspace_id,
            ip,
            title,
            path,
        })
        .await
    }

    pub(crate) async fn upload_sftp(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> ApplicationResult<SftpTransferSummary> {
        match self
            .request(ApplicationCommand::UploadSftp {
                workspace_id,
                local_paths,
            })
            .await?
        {
            ApplicationResponse::SftpTransferSummary(transfer) => Ok(transfer),
            _ => Err("MCP application bridge 返回了错误的上传响应".to_owned()),
        }
    }

    pub(crate) async fn download_sftp(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> ApplicationResult<SftpTransferSummary> {
        match self
            .request(ApplicationCommand::DownloadSftp {
                workspace_id,
                remote_paths,
            })
            .await?
        {
            ApplicationResponse::SftpTransferSummary(transfer) => Ok(transfer),
            _ => Err("MCP application bridge 返回了错误的下载响应".to_owned()),
        }
    }

    pub(crate) async fn list_sftp_transfers(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<Vec<SftpTransferInfo>> {
        match self
            .request(ApplicationCommand::ListSftpTransfers { workspace_id })
            .await?
        {
            ApplicationResponse::SftpTransferInfos(transfers) => Ok(transfers),
            _ => Err("MCP application bridge 返回了错误的传输响应".to_owned()),
        }
    }

    pub(crate) async fn watch_sftp_local(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<SftpWatchSummary> {
        match self
            .request(ApplicationCommand::WatchSftpLocal {
                workspace_id,
                ip,
                title,
                local_path,
            })
            .await?
        {
            ApplicationResponse::SftpWatch(watch) => Ok(watch),
            _ => Err("MCP application bridge 返回了错误的 watch 响应".to_owned()),
        }
    }

    pub(crate) async fn stop_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::StopSftpLocalWatch {
            workspace_id,
            ip,
            title,
            local_path,
        })
        .await
    }

    pub(crate) async fn list_sftp_local_watches(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> ApplicationResult<Vec<SftpWatchSummary>> {
        match self
            .request(ApplicationCommand::ListSftpLocalWatches {
                workspace_id,
                ip,
                title,
            })
            .await?
        {
            ApplicationResponse::SftpWatches(watches) => Ok(watches),
            _ => Err("MCP application bridge 返回了错误的 watches 响应".to_owned()),
        }
    }

    pub(crate) async fn list_terminals(&self) -> ApplicationResult<Vec<TerminalSummary>> {
        match self.request(ApplicationCommand::ListTerminals).await? {
            ApplicationResponse::TerminalSummaries(terminals) => Ok(terminals),
            _ => Err("MCP application bridge 返回了错误的终端响应".to_owned()),
        }
    }

    pub(crate) async fn select_terminal(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::SelectTerminal {
            workspace_id,
            ip,
            title,
        })
        .await
    }

    pub(crate) async fn read_terminal(
        &self,
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    ) -> ApplicationResult<TerminalReadPage> {
        match self
            .request(ApplicationCommand::ReadTerminal {
                workspace_id,
                offset,
                limit,
            })
            .await?
        {
            ApplicationResponse::TerminalRead(page) => Ok(page),
            _ => Err("MCP application bridge 返回了错误的终端读取响应".to_owned()),
        }
    }

    pub(crate) async fn send_text(
        &self,
        workspace_id: Option<String>,
        text: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::SendText { workspace_id, text })
            .await
    }

    pub(crate) async fn send_key(
        &self,
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::SendKey {
            workspace_id,
            key,
            control,
            alt,
            shift,
        })
        .await
    }

    async fn expect_empty(&self, command: ApplicationCommand) -> ApplicationResult<()> {
        match self.request(command).await? {
            ApplicationResponse::Empty => Ok(()),
            _ => Err("MCP application bridge 返回了错误的操作响应".to_owned()),
        }
    }
}

pub(crate) fn start_mcp_bridge(cx: &mut App, mut bridge: McpBridgeReceiver) {
    let application = cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
    let mut application_events = application.subscribe();
    let notification_tx = bridge.notification_tx.clone();

    cx.spawn(async move |_cx| {
        log::info!("MCP application bridge adapter started");
        loop {
            tokio::select! {
                command = bridge.command_rx.recv() => {
                    let Some(command) = command else {
                        log::info!("MCP application bridge command channel closed");
                        break;
                    };
                    let request_id = command.request_id.clone();
                    log::debug!("MCP bridge command received: request_id={request_id}");
                    let result = dispatch(&application, command.command).await;
                    if command.response_tx.send(ResponseEnvelope { request_id: request_id.clone(), result }).is_err() {
                        log::debug!("MCP bridge response receiver dropped: request_id={request_id}");
                    } else {
                        log::debug!("MCP bridge response sent: request_id={request_id}");
                    }
                }
                event = application_events.recv() => {
                    match event {
                        Ok(event) => {
                            let _ = notification_tx.send(map_event(event));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            log::warn!("MCP bridge notification receiver lagged: skipped={count}");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            application_events = application.subscribe();
                        }
                    }
                }
            }
        }
        log::info!("MCP application bridge adapter stopped");
    })
    .detach();
}

async fn dispatch(
    application: &ApplicationContext,
    command: ApplicationCommand,
) -> ApplicationResult<ApplicationResponse> {
    match command {
        ApplicationCommand::ListProfiles => application
            .list_profiles()
            .await
            .map(ApplicationResponse::Profiles),
        ApplicationCommand::OpenSession {
            profile_id,
            protocol,
            ip,
            title,
        } => application
            .open_session(profile_id, protocol, ip, title)
            .await
            .map(ApplicationResponse::WorkspaceId),
        ApplicationCommand::CloseSession { workspace_id } => application
            .close_session(&workspace_id)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ListSftpSessions => application
            .list_sftp_sessions()
            .await
            .map(ApplicationResponse::TerminalSummaries),
        ApplicationCommand::ListSftpLocal => application
            .list_sftp_local()
            .await
            .map(ApplicationResponse::SftpDirectory),
        ApplicationCommand::ChangeSftpLocalDirectory {
            workspace_id,
            ip,
            title,
            path,
        } => application
            .change_sftp_local_directory(workspace_id, ip, title, path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ListSftpRemote { workspace_id } => application
            .list_sftp_remote(workspace_id)
            .await
            .map(ApplicationResponse::SftpDirectory),
        ApplicationCommand::ChangeSftpRemoteDirectory {
            workspace_id,
            ip,
            title,
            path,
        } => application
            .change_sftp_remote_directory(workspace_id, ip, title, path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::UploadSftp {
            workspace_id,
            local_paths,
        } => application
            .upload_sftp(workspace_id, local_paths)
            .await
            .map(ApplicationResponse::SftpTransferSummary),
        ApplicationCommand::DownloadSftp {
            workspace_id,
            remote_paths,
        } => application
            .download_sftp(workspace_id, remote_paths)
            .await
            .map(ApplicationResponse::SftpTransferSummary),
        ApplicationCommand::ListSftpTransfers { workspace_id } => application
            .list_sftp_transfers(workspace_id)
            .await
            .map(ApplicationResponse::SftpTransferInfos),
        ApplicationCommand::WatchSftpLocal {
            workspace_id,
            ip,
            title,
            local_path,
        } => application
            .watch_sftp_local(workspace_id, ip, title, local_path)
            .await
            .map(ApplicationResponse::SftpWatch),
        ApplicationCommand::StopSftpLocalWatch {
            workspace_id,
            ip,
            title,
            local_path,
        } => application
            .stop_sftp_local_watch(workspace_id, ip, title, local_path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ListSftpLocalWatches {
            workspace_id,
            ip,
            title,
        } => application
            .list_sftp_local_watches(workspace_id, ip, title)
            .map(ApplicationResponse::SftpWatches),
        ApplicationCommand::ListTerminals => application
            .list_terminals()
            .await
            .map(ApplicationResponse::TerminalSummaries),
        ApplicationCommand::SelectTerminal {
            workspace_id,
            ip,
            title,
        } => application
            .select_terminal(workspace_id, ip, title)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ReadTerminal {
            workspace_id,
            offset,
            limit,
        } => application
            .read_terminal(workspace_id, offset, limit)
            .await
            .map(ApplicationResponse::TerminalRead),
        ApplicationCommand::SendText { workspace_id, text } => application
            .send_text(workspace_id, text)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::SendKey {
            workspace_id,
            key,
            control,
            alt,
            shift,
        } => application
            .send_key(workspace_id, key, control, alt, shift)
            .await
            .map(|_| ApplicationResponse::Empty),
    }
}

fn map_event(event: ApplicationEvent) -> NotificationEnvelope {
    let event = match event {
        ApplicationEvent::SessionOpened {
            workspace_id,
            profile,
        } => ApplicationNotification::SessionOpened {
            workspace_id,
            profile: ProfileSummary {
                id: profile.id,
                title: profile.name,
                host: profile.host,
                protocol: profile.protocol.as_str().to_owned(),
            },
        },
        ApplicationEvent::SessionClosed { workspace_id } => {
            ApplicationNotification::SessionClosed { workspace_id }
        }
        ApplicationEvent::SessionSelected { workspace_id } => {
            ApplicationNotification::SessionSelected { workspace_id }
        }
    };
    NotificationEnvelope { event }
}
