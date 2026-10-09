use crate::{
    application::ports::SftpConnectFuture, domain::session::SessionProfile,
    infrastructure::InfrastructureContext,
};

impl InfrastructureContext {
    pub(crate) fn connect_sftp<'a>(
        &'a self,
        workspace_id: &'a str,
        profile: &'a SessionProfile,
    ) -> SftpConnectFuture<'a> {
        super::core::connect(self, workspace_id, profile)
    }
}
