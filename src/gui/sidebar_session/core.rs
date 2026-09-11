use anyhow::{Context as _, Result};
use gpui_kit::{AppContext, Context};

use crate::{domain::session::SessionProfile, infrastructure::InfrastructureContext};

use super::SessionComponent;

impl SessionComponent {
    pub(super) fn load_sessions(&self, cx: &Context<Self>) -> Result<Vec<SessionProfile>> {
        cx.read_global::<InfrastructureContext, _>(|infrastructure, _| infrastructure.session())
            .list()
    }

    pub(super) fn query_session(&self, id: &str, cx: &Context<Self>) -> Result<SessionProfile> {
        cx.read_global::<InfrastructureContext, _>(|infrastructure, _| infrastructure.session())
            .list()?
            .into_iter()
            .find(|session| session.id == id)
            .with_context(|| format!("top_session not found: {id}"))
    }

    pub(super) fn remove_session(&self, id: &str, cx: &Context<Self>) -> Result<()> {
        cx.read_global::<InfrastructureContext, _>(|infrastructure, _| infrastructure.session())
            .delete(id)
    }
}
