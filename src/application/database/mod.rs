mod core;
mod external;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::application::ports::DatabaseConnection;

#[derive(Clone)]
pub(crate) struct DatabaseApplication {
    pub(super) inner: Arc<DatabaseApplicationInner>,
}

pub(super) struct DatabaseApplicationInner {
    pub(super) connections: RwLock<HashMap<String, Arc<dyn DatabaseConnection>>>,
}
