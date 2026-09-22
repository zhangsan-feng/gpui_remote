use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    domain::session::{NewSession, Protocol, SessionProfile},
    infrastructure::InfrastructureContext,
};
use gpui_kit::{App, AppContext, Global};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::{
    ApplicationEvent, ApplicationResult, ApplicationStoreGraph, SessionApplication,
    SftpApplication, SshApplication,
};

const APPLICATION_EVENT_CAPACITY: usize = 256;

struct ApplicationContextInner {
    infrastructure: InfrastructureContext,
    events: broadcast::Sender<ApplicationEvent>,
    sessions: SessionApplication,
    ssh: SshApplication,
    sftp: SftpApplication,
    stores: ApplicationStoreGraph,
}

enum UpdateSftpPath {
    Local(PathBuf),
    Remote(String),
}

impl From<PathBuf> for UpdateSftpPath {
    fn from(path: PathBuf) -> Self {
        Self::Local(path)
    }
}

impl From<String> for UpdateSftpPath {
    fn from(path: String) -> Self {
        Self::Remote(path)
    }
}

#[derive(Clone)]
pub struct ApplicationContext {
    inner: Arc<ApplicationContextInner>,
}

impl Global for ApplicationContext {}

impl ApplicationContext {
    pub fn new(cx: &mut App) -> Self {
        let infrastructure =
            cx.read_global::<InfrastructureContext, _>(|infrastructure, _| infrastructure.clone());
        let (events, _) = broadcast::channel(APPLICATION_EVENT_CAPACITY);
        let sessions = SessionApplication::new(events.clone());
        let ssh = SshApplication::new();
        let sftp = SftpApplication::new();
        let stores = ApplicationStoreGraph::new(cx, sessions.clone(), ssh.clone(), sftp.clone());
        Self {
            inner: Arc::new(ApplicationContextInner {
                infrastructure,
                sessions,
                ssh,
                sftp,
                stores,
                events,
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ApplicationEvent> {
        self.inner.events.subscribe()
    }

    pub fn sessions(&self) -> SessionApplication {
        self.inner.sessions.clone()
    }

    pub fn ssh(&self) -> SshApplication {
        self.inner.ssh.clone()
    }

    pub fn sftp(&self) -> SftpApplication {
        self.inner.sftp.clone()
    }

    pub async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    ) -> ApplicationResult<String> {
        let profile = self.find_session_profile(profile_id.clone()).await?;
        if profile.host != ip || profile.name != title {
            return Err(format!(
                "连接配置与 ip/title 不匹配: {profile_id}，请确认 ip 和 title"
            ));
        }
        if !profile.connection_protocol.supports(protocol) {
            return Err(format!(
                "连接类型 {} 暂不支持打开 {} 视图",
                profile.connection_protocol, protocol
            ));
        }
        let mut profile = profile;
        profile.protocol = protocol;
        let workspace_id = Uuid::new_v4().to_string();
        if self
            .inner
            .sessions
            .profile_for_workspace(&workspace_id)
            .is_some()
        {
            return Err(format!("会话已打开: {workspace_id}"));
        }

        match protocol {
            Protocol::Ssh => {
                self.inner
                    .ssh
                    .open(workspace_id.clone(), profile.clone())
                    .await?
            }
            Protocol::Sftp => {
                let session = self.inner.infrastructure.session_repository();
                let state_id = profile.id.clone();
                let state = tokio::task::spawn_blocking(move || session.read_sftp_state(&state_id))
                    .await
                    .map_err(|error| format!("读取 SFTP 会话状态任务失败: {error}"))?
                    .map_err(|error| format!("读取 SFTP 会话状态失败: {error:#}"))?
                    .unwrap_or_default();
                self.inner
                    .sftp
                    .open(
                        workspace_id.clone(),
                        profile.clone(),
                        state.remote_path,
                        state.local_path,
                    )
                    .await?;
            }
        }
        if let Err(error) = self.inner.sessions.register(workspace_id.clone(), profile) {
            match protocol {
                Protocol::Ssh => {
                    let _ = self.inner.ssh.close(&workspace_id).await;
                }
                Protocol::Sftp => {
                    let _ = self.inner.sftp.close(&workspace_id).await;
                }
            }
            return Err(error);
        }
        Ok(workspace_id)
    }

    pub async fn close_session(&self, workspace_id: &str) -> ApplicationResult<()> {
        let profile = self
            .inner
            .sessions
            .profile_for_workspace(workspace_id)
            .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
        match profile.protocol {
            Protocol::Ssh => self.inner.ssh.close(workspace_id).await?,
            Protocol::Sftp => self.inner.sftp.close(workspace_id).await?,
        }
        self.inner.sessions.close(workspace_id).await
    }

    pub async fn select_session(&self, workspace_id: Option<String>) -> ApplicationResult<()> {
        self.inner.sessions.select(workspace_id).await
    }

    pub(crate) async fn list_session_profiles(&self) -> ApplicationResult<Vec<SessionProfile>> {
        let repository = self.inner.infrastructure.session_repository();
        tokio::task::spawn_blocking(move || repository.list_sessions())
            .await
            .map_err(|error| format!("读取连接配置任务失败: {error}"))?
            .map_err(|error| format!("读取连接配置失败: {error:#}"))
    }

    pub(crate) async fn find_session_profile(
        &self,
        id: String,
    ) -> ApplicationResult<SessionProfile> {
        let repository = self.inner.infrastructure.session_repository();
        let missing_id = id.clone();
        tokio::task::spawn_blocking(move || repository.find_session(&id))
            .await
            .map_err(|error| format!("查询连接配置任务失败: {error}"))?
            .map_err(|error| format!("查询连接配置失败: {error:#}"))?
            .ok_or_else(|| format!("连接配置不存在: {missing_id}"))
    }

    pub(crate) async fn create_session(
        &self,
        draft: NewSession,
    ) -> ApplicationResult<SessionProfile> {
        let repository = self.inner.infrastructure.session_repository();
        tokio::task::spawn_blocking(move || repository.insert_session(draft))
            .await
            .map_err(|error| format!("创建连接配置任务失败: {error}"))?
            .map_err(|error| format!("创建连接配置失败: {error:#}"))
    }

    pub(crate) async fn update_session(
        &self,
        id: String,
        draft: NewSession,
    ) -> ApplicationResult<SessionProfile> {
        let repository = self.inner.infrastructure.session_repository();
        tokio::task::spawn_blocking(move || repository.update_session(&id, draft))
            .await
            .map_err(|error| format!("更新连接配置任务失败: {error}"))?
            .map_err(|error| format!("更新连接配置失败: {error:#}"))
    }

    pub(crate) async fn delete_session(&self, id: String) -> ApplicationResult<()> {
        let repository = self.inner.infrastructure.session_repository();
        tokio::task::spawn_blocking(move || repository.delete_session(&id))
            .await
            .map_err(|error| format!("删除连接配置任务失败: {error}"))?
            .map_err(|error| format!("删除连接配置失败: {error:#}"))
    }

    pub(crate) fn current_mcp_settings(&self) -> super::model::McpSettings {
        let settings = self.inner.infrastructure.current_mcp_settings();
        super::model::McpSettings {
            enabled: settings.enabled,
            token_enabled: settings.token_enabled,
            host: settings.host,
            port: settings.port,
            token: settings.token,
        }
    }

    pub(crate) async fn update_mcp_settings(
        &self,
        settings: super::model::McpSettings,
    ) -> ApplicationResult<super::model::McpSettings> {
        let infrastructure = self.inner.infrastructure.clone();
        let settings = crate::infrastructure::agent_mcp::McpSettings {
            enabled: settings.enabled,
            token_enabled: settings.token_enabled,
            host: settings.host,
            port: settings.port,
            token: settings.token,
        };
        let settings =
            tokio::task::spawn_blocking(move || infrastructure.update_mcp_settings(settings))
                .await
                .map_err(|error| format!("更新 MCP 配置任务失败: {error}"))?
                .map_err(|error| format!("更新 MCP 配置失败: {error}"))?;
        Ok(super::model::McpSettings {
            enabled: settings.enabled,
            token_enabled: settings.token_enabled,
            host: settings.host,
            port: settings.port,
            token: settings.token,
        })
    }

    pub async fn update_sftp_local_path(
        &self,
        workspace_id: &str,
        path: &Path,
    ) -> ApplicationResult<()> {
        self.update_sftp_path(workspace_id, path.to_owned(), true)
            .await
    }

    pub async fn update_sftp_remote_path(
        &self,
        workspace_id: &str,
        path: String,
    ) -> ApplicationResult<()> {
        self.update_sftp_path(workspace_id, path, false).await
    }

    async fn update_sftp_path(
        &self,
        workspace_id: &str,
        path: impl Into<UpdateSftpPath>,
        local: bool,
    ) -> ApplicationResult<()> {
        let profile = self
            .inner
            .sessions
            .profile_for_workspace(workspace_id)
            .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
        if profile.protocol != Protocol::Sftp {
            return Err(format!("会话协议不是 SFTP: {workspace_id}"));
        }
        let session = self.inner.infrastructure.session_repository();
        let profile_id = profile.id;
        let path = path.into();
        tokio::task::spawn_blocking(move || match (local, path) {
            (true, UpdateSftpPath::Local(path)) => session
                .update_sftp_local_path(&profile_id, &path)
                .map_err(|error| format!("保存 SFTP 本地目录失败: {error:#}")),
            (false, UpdateSftpPath::Remote(path)) => session
                .update_sftp_remote_path(&profile_id, &path)
                .map_err(|error| format!("保存 SFTP 远程目录失败: {error:#}")),
            _ => Err("SFTP 路径类型不匹配".to_owned()),
        })
        .await
        .map_err(|error| format!("保存 SFTP 目录任务失败: {error}"))?
    }
}
