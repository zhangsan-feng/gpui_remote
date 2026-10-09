mod core;
mod external;
mod mapping;
pub(crate) mod model;
pub(crate) mod validation;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::domain::session::SessionProfile;

struct SessionState {
    profiles: RwLock<HashMap<String, SessionProfile>>,
}

#[derive(Clone)]
pub struct SessionApplication {
    inner: Arc<SessionState>,
}
