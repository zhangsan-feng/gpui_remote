use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio::sync::broadcast;

use crate::domain::session::SessionProfile;

use super::{ApplicationEvent, ApplicationResult};

struct SessionState {
    sessions: RwLock<HashMap<String, SessionProfile>>,
    session_order: RwLock<Vec<String>>,
    selected_workspace_id: RwLock<Option<String>>,
    events: broadcast::Sender<ApplicationEvent>,
}

#[derive(Clone)]
pub struct SessionApplication {
    inner: Arc<SessionState>,
}

impl SessionApplication {
    pub(crate) fn new(events: broadcast::Sender<ApplicationEvent>) -> Self {
        Self {
            inner: Arc::new(SessionState {
                sessions: RwLock::new(HashMap::new()),
                session_order: RwLock::new(Vec::new()),
                selected_workspace_id: RwLock::new(None),
                events,
            }),
        }
    }

    pub(crate) fn register(
        &self,
        workspace_id: String,
        profile: SessionProfile,
    ) -> ApplicationResult<()> {
        {
            let mut sessions = self
                .inner
                .sessions
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if sessions
                .insert(workspace_id.clone(), profile.clone())
                .is_some()
            {
                return Err(format!("会话已打开: {workspace_id}"));
            }
            self.inner
                .session_order
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(workspace_id.clone());
        }

        let _ = self.inner.events.send(ApplicationEvent::SessionOpened {
            workspace_id: workspace_id.clone(),
            profile,
        });
        self.select_sync(Some(workspace_id))
    }

    pub async fn close(&self, workspace_id: &str) -> ApplicationResult<()> {
        let (was_selected, selected_after_close) = {
            let mut sessions = self
                .inner
                .sessions
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if sessions.remove(workspace_id).is_none() {
                return Err(format!("会话不存在: {workspace_id}"));
            }

            let mut session_order = self
                .inner
                .session_order
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let closed_index = session_order
                .iter()
                .position(|session_id| session_id == workspace_id);
            if let Some(closed_index) = closed_index {
                session_order.remove(closed_index);
            }

            let was_selected = self
                .inner
                .selected_workspace_id
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_deref()
                == Some(workspace_id);
            let selected_after_close = was_selected.then(|| {
                closed_index
                    .and_then(|index| {
                        session_order.get(index.min(session_order.len().saturating_sub(1)))
                    })
                    .cloned()
            });
            (was_selected, selected_after_close.flatten())
        };
        let _ = self.inner.events.send(ApplicationEvent::SessionClosed {
            workspace_id: workspace_id.to_owned(),
        });
        if was_selected {
            self.select_sync(selected_after_close)?;
        }
        Ok(())
    }

    pub async fn select(&self, workspace_id: Option<String>) -> ApplicationResult<()> {
        self.select_sync(workspace_id)
    }

    fn select_sync(&self, workspace_id: Option<String>) -> ApplicationResult<()> {
        if let Some(workspace_id) = workspace_id.as_deref() {
            let exists = self
                .inner
                .sessions
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .contains_key(workspace_id);
            if !exists {
                return Err(format!("会话不存在: {workspace_id}"));
            }
        }

        let mut selected_workspace_id = self
            .inner
            .selected_workspace_id
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *selected_workspace_id == workspace_id {
            return Ok(());
        }
        *selected_workspace_id = workspace_id.clone();
        drop(selected_workspace_id);

        let _ = self
            .inner
            .events
            .send(ApplicationEvent::SessionSelected { workspace_id });
        Ok(())
    }

    pub fn profile_for_workspace(&self, workspace_id: &str) -> Option<SessionProfile> {
        self.inner
            .sessions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .cloned()
    }

    pub fn list_workspaces(&self) -> Vec<(String, SessionProfile)> {
        let mut sessions = self
            .inner
            .sessions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(|(workspace_id, profile)| (workspace_id.clone(), profile.clone()))
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.0.cmp(&right.0));
        sessions
    }

    pub fn selected_workspace_id(&self) -> Option<String> {
        self.inner
            .selected_workspace_id
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}
