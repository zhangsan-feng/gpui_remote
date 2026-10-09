use crate::{
    application::ports::DatabaseConnectFuture, domain::session::SessionProfile,
    infrastructure::InfrastructureContext,
};

impl InfrastructureContext {
    pub(crate) fn connect_database<'a>(
        &'a self,
        profile: &'a SessionProfile,
    ) -> DatabaseConnectFuture<'a> {
        Box::pin(async move {
            anyhow::bail!("{} 数据库驱动尚未接入", profile.connection_protocol)
        })
    }
}
