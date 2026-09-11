use std::sync::Arc;

use crate::{
    application::model::ProfileSummary, infrastructure::storage::SessionStorageRepository,
};

pub(crate) trait ProfileQuery: Send + Sync + 'static {
    fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String>;
}

#[derive(Clone)]
pub(crate) struct QueryService {
    profile_query: Arc<dyn ProfileQuery>,
}

impl QueryService {
    pub(crate) fn new(profile_query: Arc<dyn ProfileQuery>) -> Self {
        Self { profile_query }
    }

    pub(crate) async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
        let started_at = std::time::Instant::now();
        let profile_query = Arc::clone(&self.profile_query);
        log::debug!("infrastructure profile query started: command=profiles.list");
        let result = tokio::task::spawn_blocking(move || profile_query.list_profiles())
            .await
            .map_err(|error| format!("读取连接配置任务失败: {error}"))?;
        match &result {
            Ok(profiles) => log::debug!(
                "infrastructure profile query finished: command=profiles.list, count={}, elapsed_ms={}",
                profiles.len(),
                started_at.elapsed().as_millis()
            ),
            Err(error) => log::warn!(
                "infrastructure profile query failed: elapsed_ms={}, error={error}",
                started_at.elapsed().as_millis()
            ),
        }
        result
    }
}

pub(crate) fn sqlite(session: SessionStorageRepository) -> Arc<dyn ProfileQuery> {
    Arc::new(SqliteProfileQuery { session })
}

struct SqliteProfileQuery {
    session: SessionStorageRepository,
}

impl ProfileQuery for SqliteProfileQuery {
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
