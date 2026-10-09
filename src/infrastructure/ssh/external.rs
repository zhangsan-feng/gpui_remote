use std::collections::HashSet;

use crate::{
    application::ports::SshOpenFuture, domain::session::SessionProfile,
    infrastructure::InfrastructureContext,
};

impl InfrastructureContext {
    pub(crate) fn open_ssh_shell<'a>(
        &'a self,
        workspace_id: &'a str,
        profile: &'a SessionProfile,
    ) -> SshOpenFuture<'a> {
        super::core::open_shell(self, workspace_id, profile)
    }

    pub(crate) async fn open_ssh_reverse_tunnel(
        &self,
        profile: &SessionProfile,
    ) -> anyhow::Result<()> {
        self.inner.ssh_reverse_tunnel.open(self, profile).await
    }

    pub(crate) async fn close_ssh_reverse_tunnel(&self, profile_id: &str) {
        self.inner.ssh_reverse_tunnel.close(profile_id).await;
    }

    pub(crate) async fn active_ssh_reverse_tunnel_ids(&self) -> HashSet<String> {
        self.inner.ssh_reverse_tunnel.active_ids().await
    }
}
