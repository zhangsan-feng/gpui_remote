use crate::{
    application::{ports::SshServerFuture, sftp_server::SftpServerApplication},
    domain::ssh_server::{SshServerSettings, SshServerStatus},
    infrastructure::InfrastructureContext,
};

impl InfrastructureContext {
    pub(crate) fn load_ssh_server_settings(&self) -> SshServerFuture<SshServerSettings> {
        let context = self.clone();
        Box::pin(async move { context.inner.ssh_server.load_settings().await })
    }

    pub(crate) fn start_ssh_server(
        &self,
        settings: SshServerSettings,
        sftp_server: std::sync::Arc<SftpServerApplication>,
    ) -> SshServerFuture<SshServerStatus> {
        let context = self.clone();
        Box::pin(async move { context.inner.ssh_server.start(settings, sftp_server).await })
    }

    pub(crate) fn update_ssh_server(
        &self,
        settings: SshServerSettings,
        sftp_server: std::sync::Arc<SftpServerApplication>,
    ) -> SshServerFuture<SshServerStatus> {
        let context = self.clone();
        Box::pin(async move { context.inner.ssh_server.update(settings, sftp_server).await })
    }

    pub(crate) fn shutdown_ssh_server(&self) -> SshServerFuture<()> {
        let context = self.clone();
        Box::pin(async move {
            context.inner.ssh_server.shutdown().await;
            Ok(())
        })
    }

    pub(crate) fn subscribe_ssh_server_status(
        &self,
    ) -> tokio::sync::watch::Receiver<SshServerStatus> {
        self.inner.ssh_server.subscribe_status()
    }
}
