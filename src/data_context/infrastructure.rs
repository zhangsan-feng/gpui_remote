use std::sync::Arc;

use super::query::{ProfileQuery, QueryService};

#[derive(Clone)]
pub struct InfrastructureContext {
    profile_query: Arc<dyn ProfileQuery>,
}

impl InfrastructureContext {
    pub fn new(profile_query: Arc<dyn ProfileQuery>) -> Self {
        Self { profile_query }
    }

    pub(crate) fn query_service(&self) -> QueryService {
        QueryService::new(Arc::clone(&self.profile_query))
    }
}
