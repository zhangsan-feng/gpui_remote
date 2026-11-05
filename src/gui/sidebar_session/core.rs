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
            let result = crate::application::run_application(async move {
                application.sessions.list_session_summaries().await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                if this.refresh_generation != refresh_generation {
                    return;
                }
                match result {
                    Ok(mut sessions) => {
                        sessions.sort_by_key(|session| {
                            this.sessions
                                .iter()
                                .position(|old| old.id == session.id)
                                .unwrap_or(usize::MAX)
                        });
                        this.sessions = sessions;
                        this.core_err = None;
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

impl SessionComponent {
    pub(super) fn move_session(&mut self, from: &str, target: &str, cx: &mut Context<Self>) {
        let Some(from_index) = self.sessions.iter().position(|session| session.id == from) else {
            return;
        };
        let Some(target_index) = self
            .sessions
            .iter()
            .position(|session| session.id == target)
        else {
            return;
        };
        let after = self
            .drop_target
            .as_ref()
            .filter(|(id, _)| id == target)
            .is_some_and(|(_, after)| *after);
        self.drop_target = None;
        if from_index != target_index {
            let insertion = target_index + usize::from(after);
            let to = insertion - usize::from(from_index < insertion);
            let session = self.sessions.remove(from_index);
            self.sessions.insert(to, session);
        }
        cx.notify();
    }
}

impl SessionComponent {
    pub(super) fn set_drop_target(
        &mut self,
        target: Option<(String, bool)>,
        cx: &mut Context<Self>,
    ) {
        if self.drop_target != target {
            self.drop_target = target;
            cx.notify();
        }
    }
}
