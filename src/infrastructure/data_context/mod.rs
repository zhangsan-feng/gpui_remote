mod profile_query;

use crate::{
    data_context::InfrastructureContext, infrastructure::storage::SessionStorageRepository,
};

pub(super) fn new(session: SessionStorageRepository) -> InfrastructureContext {
    InfrastructureContext::new(profile_query::new(session))
}
