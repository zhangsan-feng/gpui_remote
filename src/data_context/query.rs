use std::sync::Arc;

use super::model::ProfileSummary;

pub trait ProfileQuery: Send + Sync + 'static {
    fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String>;
}

#[derive(Clone)]
pub struct QueryService {
    profile_query: Arc<dyn ProfileQuery>,
}

impl QueryService {
    pub fn new(profile_query: Arc<dyn ProfileQuery>) -> Self {
        Self { profile_query }
    }

    pub async fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
        let started_at = std::time::Instant::now();
        let profile_query = Arc::clone(&self.profile_query);
        log::debug!("DataContext query started: command=profiles.list");
        let result = tokio::task::spawn_blocking(move || profile_query.list_profiles())
            .await
            .map_err(|error| format!("DataContext profile query task failed: {error}"))?;
        match &result {
            Ok(profiles) => log::debug!(
                "DataContext query finished: command=profiles.list, count={}, elapsed_ms={}",
                profiles.len(),
                started_at.elapsed().as_millis()
            ),
            Err(error) => log::warn!(
                "DataContext query failed: command=profiles.list, elapsed_ms={}, error={error}",
                started_at.elapsed().as_millis()
            ),
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{ProfileQuery, QueryService};
    use crate::data_context::ProfileSummary;

    struct StaticProfileQuery;

    impl ProfileQuery for StaticProfileQuery {
        fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
            Ok(vec![ProfileSummary {
                id: "profile-1".to_owned(),
                title: "Local".to_owned(),
                host: "127.0.0.1".to_owned(),
                protocol: "ssh".to_owned(),
            }])
        }
    }

    #[tokio::test]
    async fn list_profiles_runs_through_the_query_service() {
        let service = QueryService::new(Arc::new(StaticProfileQuery));

        let profiles = service
            .list_profiles()
            .await
            .expect("profile query should succeed");

        assert_eq!(profiles[0].id, "profile-1");
    }
}
