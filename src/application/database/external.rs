use super::DatabaseApplication;

impl DatabaseApplication {
    pub(crate) fn runtime_available(&self, workspace_id: &str) -> bool {
        self.inner
            .connections
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(workspace_id)
    }
}
