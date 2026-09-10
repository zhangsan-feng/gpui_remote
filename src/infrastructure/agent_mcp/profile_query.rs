use std::sync::Arc;

use crate::{
    application::agent_mcp::{AgentMcpProfileQuery, ProfileSummary},
    infrastructure::storage::SessionStorageRepository,
};

pub(super) fn new(session: SessionStorageRepository) -> Arc<dyn AgentMcpProfileQuery> {
    Arc::new(SqliteProfileQuery { session })
}

struct SqliteProfileQuery {
    session: SessionStorageRepository,
}

impl AgentMcpProfileQuery for SqliteProfileQuery {
    fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
        self.session
            .list()
            .map(|profiles| {
                profiles
                    .into_iter()
                    .map(|profile| ProfileSummary {
                        id: profile.id,
                        title: profile.name,
                        host: profile.host,
                        protocol: profile.protocol.as_str().to_owned(),
                    })
                    .collect()
            })
            .map_err(|error| format!("读取连接配置失败: {error:#}"))
    }
}
