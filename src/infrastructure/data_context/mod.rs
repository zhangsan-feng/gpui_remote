mod profile_query;

use crate::{
    data_context::InfrastructureContext, infrastructure::storage::SessionStorageRepository,
};

pub(super) fn new(session: SessionStorageRepository) -> InfrastructureContext {
    InfrastructureContext::with_session(profile_query::new(session.clone()), session)
}
