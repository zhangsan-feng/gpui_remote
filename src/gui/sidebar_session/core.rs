use anyhow::{Context as _, Result};
use gpui::Context;

use crate::{domain::session::SessionProfile, infrastructure::storage::Storage};

use super::SessionComponent;

impl SessionComponent {
    pub(super) fn load_sessions(&self, cx: &Context<Self>) -> Result<Vec<SessionProfile>> {
        cx.global::<Storage>().session.list()
    }

    pub(super) fn query_session(&self, id: &str, cx: &Context<Self>) -> Result<SessionProfile> {
        cx.global::<Storage>()
            .session
            .list()?
            .into_iter()
            .find(|session| session.id == id)
            .with_context(|| format!("top_session not found: {id}"))
    }

    pub(super) fn remove_session(&self, id: &str, cx: &Context<Self>) -> Result<()> {
        cx.global::<Storage>().session.delete(id)
    }
}
