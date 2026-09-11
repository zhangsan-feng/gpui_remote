use std::sync::Arc;

use gpui_kit::Global;

use super::{profile_query, profile_query::QueryService, storage::SessionStorageRepository};

#[derive(Clone)]
pub struct InfrastructureContext {
    profile_query: Arc<dyn profile_query::ProfileQuery>,
    session: Option<SessionStorageRepository>,
}

impl Global for InfrastructureContext {}

impl InfrastructureContext {
    pub(crate) fn new(session: SessionStorageRepository) -> Self {
        Self {
            profile_query: profile_query::sqlite(session.clone()),
            session: Some(session),
        }
    }

    pub(crate) fn from_profile_query(profile_query: Arc<dyn profile_query::ProfileQuery>) -> Self {
        Self {
            profile_query,
            session: None,
        }
    }

    pub(crate) fn query_service(&self) -> QueryService {
        QueryService::new(Arc::clone(&self.profile_query))
    }

    pub(crate) fn session(&self) -> Option<SessionStorageRepository> {
        self.session.clone()
    }
}
