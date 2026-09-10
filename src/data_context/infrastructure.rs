use std::sync::Arc;

use crate::infrastructure::storage::SessionStorageRepository;

use super::query::{ProfileQuery, QueryService};

#[derive(Clone)]
pub struct InfrastructureContext {
    profile_query: Arc<dyn ProfileQuery>,
    session: Option<SessionStorageRepository>,
}

impl InfrastructureContext {
    pub fn new(profile_query: Arc<dyn ProfileQuery>) -> Self {
        Self {
            profile_query,
            session: None,
        }
    }

    pub(crate) fn with_session(
        profile_query: Arc<dyn ProfileQuery>,
        session: SessionStorageRepository,
    ) -> Self {
        Self {
            profile_query,
            session: Some(session),
        }
    }

    pub(crate) fn query_service(&self) -> QueryService {
        QueryService::new(Arc::clone(&self.profile_query))
    }

    pub(crate) fn session(&self) -> Option<SessionStorageRepository> {
        self.session.clone()
    }
}
