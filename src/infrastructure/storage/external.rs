use crate::{
    application::ports::SessionRepositoryFuture,
    domain::session::{NewSession, SessionProfile, SftpSessionState},
    infrastructure::InfrastructureContext,
};

use super::Storage;

impl InfrastructureContext {
    pub(super) fn storage(&self) -> anyhow::Result<&Storage> {
        self.inner
            .storage
            .get()
            .ok_or_else(|| anyhow::anyhow!("基础设施尚未初始化"))
    }

    pub(crate) fn list_session_profiles(&self) -> SessionRepositoryFuture<Vec<SessionProfile>> {
        let context = self.clone();
        Box::pin(async move {
            let repository = context.storage()?.session.clone();
            tokio::task::spawn_blocking(move || repository.list_sessions()).await?
        })
    }

    pub(crate) fn find_session_profile(
        &self,
        id: String,
    ) -> SessionRepositoryFuture<Option<SessionProfile>> {
        let context = self.clone();
        Box::pin(async move {
            let repository = context.storage()?.session.clone();
            tokio::task::spawn_blocking(move || repository.find_session(&id)).await?
        })
    }

    pub(crate) fn create_session(
        &self,
        draft: NewSession,
    ) -> SessionRepositoryFuture<SessionProfile> {
        let context = self.clone();
        Box::pin(async move {
            let repository = context.storage()?.session.clone();
            tokio::task::spawn_blocking(move || repository.insert_session(draft)).await?
        })
    }

    pub(crate) fn update_session(
        &self,
        id: String,
        draft: NewSession,
    ) -> SessionRepositoryFuture<SessionProfile> {
        let context = self.clone();
        Box::pin(async move {
            let repository = context.storage()?.session.clone();
            tokio::task::spawn_blocking(move || repository.update_session(&id, draft)).await?
        })
    }

    pub(crate) fn delete_session(&self, id: String) -> SessionRepositoryFuture<()> {
        let context = self.clone();
        Box::pin(async move {
            let repository = context.storage()?.session.clone();
            tokio::task::spawn_blocking(move || repository.delete_session(&id)).await?
        })
    }

    pub(crate) fn read_sftp_state(
        &self,
        id: String,
    ) -> SessionRepositoryFuture<Option<SftpSessionState>> {
        let context = self.clone();
        Box::pin(async move {
            let repository = context.storage()?.session.clone();
            tokio::task::spawn_blocking(move || repository.read_sftp_state(&id)).await?
        })
    }

    pub(crate) fn update_sftp_local_path(
        &self,
        id: String,
        path: std::path::PathBuf,
    ) -> SessionRepositoryFuture<()> {
        let context = self.clone();
        Box::pin(async move {
            let repository = context.storage()?.session.clone();
            tokio::task::spawn_blocking(move || repository.update_sftp_local_path(&id, &path))
                .await?
        })
    }

    pub(crate) fn update_sftp_remote_path(
        &self,
        id: String,
        path: String,
    ) -> SessionRepositoryFuture<()> {
        let context = self.clone();
        Box::pin(async move {
            let repository = context.storage()?.session.clone();
            tokio::task::spawn_blocking(move || repository.update_sftp_remote_path(&id, &path))
                .await?
        })
    }
}
