use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    application::{Application, ApplicationResult},
    data_context::{DATA_CONTEXT, WorkspaceSummary},
    domain::{
        session::{Protocol, SessionProfile},
        terminal::TerminalStatus,
    },
    infrastructure::INFRASTRUCTURE,
};
use uuid::Uuid;

use super::{SessionApplication, SessionState, model::SshTunnelWorkspaceSummary};

impl SessionApplication {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(SessionState {
                profiles: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub(crate) fn register(
        &self,
        workspace_id: String,
        profile: SessionProfile,
    ) -> ApplicationResult<()> {
        let mut profiles = self
            .inner
            .profiles
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if profiles.contains_key(&workspace_id) {
            return Err(format!("会话已打开: {workspace_id}"));
        }
        profiles.insert(workspace_id.clone(), profile.clone());
        let summary = WorkspaceSummary {
            workspace_id: workspace_id.clone(),
            profile_id: profile.id,
            connection_protocol: profile.connection_protocol,
            title: profile.name,
            host: profile.host,
            protocol: profile.protocol,
            status: TerminalStatus::Connecting,
            terminal_revision: 0,
            sftp_revision: 0,
            database_revision: 0,
        };
        if let Err(error) = DATA_CONTEXT.open_workspace(summary) {
            profiles.remove(&workspace_id);
            return Err(error);
        }
        Ok(())
    }

    pub async fn close(&self, workspace_id: &str) -> ApplicationResult<()> {
        let mut profiles = self
            .inner
            .profiles
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let profile = profiles
            .remove(workspace_id)
            .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
        if let Err(error) = DATA_CONTEXT.close_workspace(workspace_id) {
            profiles.insert(workspace_id.to_owned(), profile);
            return Err(error);
        }
        Ok(())
    }

    pub async fn select(&self, workspace_id: Option<String>) -> ApplicationResult<()> {
        DATA_CONTEXT.select_workspace(workspace_id)
    }

    pub fn profile_for_workspace(&self, workspace_id: &str) -> Option<SessionProfile> {
        self.inner
            .profiles
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .cloned()
    }
}

impl Application {
    pub(crate) async fn open_ssh_tunnel(
        &self,
        profile_id: String,
    ) -> ApplicationResult<SshTunnelWorkspaceSummary> {
        let profile = self.sessions.find_profile(profile_id.clone()).await?;
        let remote_port = profile
            .ssh_reverse_tunnel
            .as_ref()
            .map(|tunnel| tunnel.remote_port)
            .ok_or_else(|| "请先在连接配置的 SSH 隧道页填写转发参数".to_owned())?;

        self.ssh.open_reverse_tunnel(profile_id.clone()).await?;
        let workspace_id = self
            .open_session(
                profile_id.clone(),
                Protocol::Ssh,
                profile.host.clone(),
                profile.name.clone(),
                true,
            )
            .await?;

        Ok(SshTunnelWorkspaceSummary {
            workspace_id,
            profile_id,
            ip: profile.host,
            title: profile.name,
            remote_port,
        })
    }

    pub async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
        send_tunnel_proxy_exports: bool,
    ) -> ApplicationResult<String> {
        let profile = self.sessions.find_profile(profile_id.clone()).await?;
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
        let tunnel_proxy_exports = if send_tunnel_proxy_exports {
            if protocol != Protocol::Ssh {
                return Err("SSH 隧道代理环境变量只能发送到 SSH 终端".to_owned());
            }
            if !INFRASTRUCTURE
                .active_ssh_reverse_tunnel_ids()
                .await
                .contains(&profile.id)
            {
                return Err("SSH 隧道尚未启动，无法设置代理环境变量".to_owned());
            }
            let tunnel = profile
                .ssh_reverse_tunnel
                .as_ref()
                .ok_or_else(|| "连接配置中没有 SSH 隧道参数".to_owned())?;
            Some(tunnel_proxy_export_commands(tunnel.remote_port))
        } else {
            None
        };
        let mut profile = profile;
        profile.protocol = protocol;
        let workspace_id = Uuid::new_v4().to_string();
        if self.sessions.profile_for_workspace(&workspace_id).is_some() {
            return Err(format!("会话已打开: {workspace_id}"));
        }

        let sftp_state = if protocol == Protocol::Sftp {
            Some(
                INFRASTRUCTURE
                    .read_sftp_state(profile.id.clone())
                    .await
                    .map_err(|error| format!("读取 SFTP 会话状态失败: {error:#}"))?
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        self.sessions
            .register(workspace_id.clone(), profile.clone())?;
        let open_result = match protocol {
            Protocol::Ssh => self.ssh.open(workspace_id.clone(), profile).await,
            Protocol::Sftp => {
                let state = sftp_state.expect("SFTP state loaded before registration");
                self.sftp
                    .open(
                        workspace_id.clone(),
                        profile,
                        state.remote_path,
                        state.local_path,
                    )
                    .await
            }
            Protocol::Mysql | Protocol::Pgsql | Protocol::Redis => {
                self.database.open(workspace_id.clone(), profile).await
            }
        };
        if let Err(error) = open_result {
            match protocol {
                Protocol::Ssh => {
                    let _ = self.ssh.close(&workspace_id).await;
                }
                Protocol::Sftp => {
                    let _ = self.sftp.close(&workspace_id).await;
                }
                Protocol::Mysql | Protocol::Pgsql | Protocol::Redis => {
                    let _ = self.database.close(&workspace_id).await;
                }
            }
            let _ = self.sessions.close(&workspace_id).await;
            return Err(error);
        }
        if let Some(commands) = tunnel_proxy_exports
            && let Err(error) = self.ssh.send_input(&workspace_id, commands).await
        {
            let _ = self.ssh.close(&workspace_id).await;
            let _ = self.sessions.close(&workspace_id).await;
            return Err(format!("发送 SSH 隧道代理环境变量失败: {error}"));
        }
        Ok(workspace_id)
    }

    pub async fn close_session(&self, workspace_id: &str) -> ApplicationResult<()> {
        let profile = self
            .sessions
            .profile_for_workspace(workspace_id)
            .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
        if matches!(
            profile.protocol,
            Protocol::Mysql | Protocol::Pgsql | Protocol::Redis
        ) {
            let close_result = self.database.close(workspace_id).await;
            let session_result = self.sessions.close(workspace_id).await;
            return match (close_result, session_result) {
                (Ok(()), Ok(())) => Ok(()),
                (Err(close_error), Ok(())) => Err(close_error),
                (Ok(()), Err(session_error)) => Err(session_error),
                (Err(close_error), Err(session_error)) => Err(format!(
                    "关闭数据库连接失败，且清理会话状态失败: {close_error}; {session_error}"
                )),
            };
        }
        if profile.protocol == Protocol::Sftp {
            // Revoke the published workspace before cleanup so pending SFTP IO
            // cannot make a closing workspace appear active again.
            self.sessions.close(workspace_id).await?;
            return self.sftp.close(workspace_id).await;
        }
        match profile.protocol {
            Protocol::Ssh => self.ssh.close(workspace_id).await?,
            Protocol::Sftp | Protocol::Mysql | Protocol::Pgsql | Protocol::Redis => unreachable!(),
        }
        self.sessions.close(workspace_id).await
    }
}

fn tunnel_proxy_export_commands(remote_port: u16) -> Vec<u8> {
    let proxy_url = shell_single_quote(&format!("http://127.0.0.1:{remote_port}"));
    format!("export http_proxy={proxy_url}\nexport https_proxy={proxy_url}\n").into_bytes()
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
