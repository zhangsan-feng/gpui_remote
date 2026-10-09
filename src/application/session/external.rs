use crate::{
    application::{Application, ApplicationResult},
    domain::session::{NewSession, Protocol, SessionProfile},
    infrastructure::INFRASTRUCTURE,
};

use super::SessionApplication;
use super::mapping;

impl SessionApplication {
    pub(crate) async fn list_profiles(&self) -> ApplicationResult<Vec<SessionProfile>> {
        INFRASTRUCTURE
            .list_session_profiles()
            .await
            .map_err(|error| format!("读取连接配置失败: {error:#}"))
    }

    pub(crate) async fn find_profile(&self, id: String) -> ApplicationResult<SessionProfile> {
        let missing_id = id.clone();
        INFRASTRUCTURE
            .find_session_profile(id)
            .await
            .map_err(|error| format!("查询连接配置失败: {error:#}"))?
            .ok_or_else(|| format!("连接配置不存在: {missing_id}"))
    }

    pub(crate) async fn create_profile(
        &self,
        draft: NewSession,
    ) -> ApplicationResult<SessionProfile> {
        draft.validate().map_err(str::to_owned)?;
        INFRASTRUCTURE
            .create_session(draft)
            .await
            .map_err(|error| format!("创建连接配置失败: {error:#}"))
    }

    pub(crate) async fn update_profile(
        &self,
        id: String,
        draft: NewSession,
    ) -> ApplicationResult<SessionProfile> {
        draft.validate().map_err(str::to_owned)?;
        INFRASTRUCTURE.close_ssh_reverse_tunnel(&id).await;
        INFRASTRUCTURE
            .update_session(id, draft)
            .await
            .map_err(|error| format!("更新连接配置失败: {error:#}"))
    }

    pub(crate) async fn delete_profile(&self, id: String) -> ApplicationResult<()> {
        self.find_profile(id.clone()).await?;
        INFRASTRUCTURE.close_ssh_reverse_tunnel(&id).await;
        INFRASTRUCTURE
            .delete_session(id)
            .await
            .map_err(|error| format!("删除连接配置失败: {error:#}"))
    }

    pub(crate) async fn list_profile_summaries(
        &self,
    ) -> ApplicationResult<Vec<super::model::ProfileSummary>> {
        self.list_profiles().await.map(|profiles| {
            profiles
                .into_iter()
                .map(mapping::map_profile_summary)
                .collect()
        })
    }

    pub(crate) async fn list_session_summaries(
        &self,
    ) -> ApplicationResult<Vec<super::model::SessionSummary>> {
        let active_tunnels = INFRASTRUCTURE.active_ssh_reverse_tunnel_ids().await;
        self.list_profiles().await.map(|profiles| {
            profiles
                .into_iter()
                .map(|profile| {
                    let tunnel_active = active_tunnels.contains(&profile.id);
                    mapping::map_session_summary(profile, tunnel_active)
                })
                .collect()
        })
    }
}

impl Application {
    pub(crate) fn workspace_runtime_available(&self, workspace_id: &str) -> bool {
        let Some(profile) = self.sessions.profile_for_workspace(workspace_id) else {
            return false;
        };
        match profile.protocol {
            Protocol::Ssh => self.ssh.runtime_available(workspace_id),
            Protocol::Sftp => self.sftp.runtime_available(workspace_id),
            Protocol::Mysql | Protocol::Pgsql | Protocol::Redis => {
                self.database.runtime_available(workspace_id)
            }
        }
    }
}
