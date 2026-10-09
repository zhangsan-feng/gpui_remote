use gpui_kit::{App, Entity, EventEmitter, Global};

use crate::domain::session::Protocol;

/// Run an owned application operation on Tokio while GPUI awaits its result.
pub async fn run_application<T: Send + 'static>(
    operation: impl std::future::Future<Output = crate::application::ApplicationResult<T>>
    + Send
    + 'static,
) -> crate::application::ApplicationResult<T> {
    tokio::spawn(async move {
        let _accepted = crate::application::APPLICATION.begin_operation().await?;
        operation.await
    })
    .await
    .map_err(|error| format!("应用层后台任务失败: {error}"))?
}

#[derive(Clone, Debug)]
pub enum GlobalEvent {
    CreateSession,
    UpdateSession,
    OpenWorkspaceSession {
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
        send_tunnel_proxy_exports: bool,
    },
    SelectWorkspaceSession(Option<String>),
    CloseWorkspaceSession {
        workspace_id: String,
    },
    ThemeColorChanged,
}

pub struct GlobalState {}

impl EventEmitter<GlobalEvent> for GlobalState {}
pub struct GlobalStateHandle(pub Entity<GlobalState>);
impl Global for GlobalStateHandle {}

pub fn read_global_state(cx: &App) -> Entity<GlobalState> {
    cx.global::<GlobalStateHandle>().0.clone()
}
