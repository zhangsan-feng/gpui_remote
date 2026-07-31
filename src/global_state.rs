use gpui::{App, Entity, EventEmitter, Global};

use crate::domain::session::SessionProfile;

#[derive(Clone, Debug)]
pub enum GlobalEvent {
    CreateSession,
    UpdateSession,
    ThemeChanged,
    OpenWorkspaceSession(String, SessionProfile),
    SelectWorkspaceSession(Option<String>),
    CloseWorkspaceSession { workspace_id: String },
}

pub struct GlobalState {}

impl EventEmitter<GlobalEvent> for GlobalState {}
pub struct GlobalStateHandle(pub Entity<GlobalState>);
impl Global for GlobalStateHandle {}

pub fn read_global_state(cx: &App) -> Entity<GlobalState> {
    cx.global::<GlobalStateHandle>().0.clone()
}
