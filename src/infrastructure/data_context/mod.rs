use crate::{
    infrastructure::InfrastructureContext, infrastructure::storage::SessionStorageRepository,
};

pub(super) fn new(session: SessionStorageRepository) -> InfrastructureContext {
    InfrastructureContext::new(session)
}
