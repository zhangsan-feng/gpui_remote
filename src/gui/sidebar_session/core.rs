use anyhow::{Context as _, Result, anyhow};
use gpui_kit::Context;

use crate::application::session::model::SessionSummary;

use super::SessionComponent;

impl SessionComponent {
    pub(super) fn refresh_sessions(&mut self, cx: &mut Context<Self>) {
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
        let refresh_generation = self.refresh_generation;
        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.sessions.list_session_summaries().await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                if this.refresh_generation != refresh_generation {
                    return;
                }
                match result {
                    Ok(sessions) => {
                        this.sessions = sessions;
                        this.core_err = None;
                        this.render_item(cx);
                        this.refer_item(cx);
                        cx.notify();
                    }
                    Err(error) => this.set_error(anyhow!(error), cx),
                }
            });
        })
        .detach();
    }

    pub(super) fn find_session_in_projection(&self, id: &str) -> Result<SessionSummary> {
        self.sessions
            .iter()
            .into_iter()
            .find(|session| session.id == id)
            .with_context(|| format!("top_session not found: {id}"))
            .cloned()
    }
}
