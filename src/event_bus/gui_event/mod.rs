mod external;
use crate::domain::session::Protocol;
use gpui_kit::EntityId;
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GuiTarget {
    Workspace,
    SidebarSession,
}

#[derive(Clone, Debug)]
pub(crate) enum GuiEvent {
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

impl GuiEvent {
    fn target(&self) -> GuiTarget {
        match self {
            Self::CreateSession | Self::UpdateSession | Self::ThemeColorChanged => {
                GuiTarget::SidebarSession
            }
            Self::OpenWorkspaceSession { .. }
            | Self::SelectWorkspaceSession(..)
            | Self::CloseWorkspaceSession { .. } => GuiTarget::Workspace,
        }
    }
}

#[derive(Default)]
pub(crate) struct GuiEventBus {
    mailboxes: Mutex<HashMap<(EntityId, GuiTarget), mpsc::UnboundedSender<GuiEvent>>>,
}

pub(crate) struct GuiEventSubscription {
    scope: EntityId,
    target: GuiTarget,
    sender: mpsc::UnboundedSender<GuiEvent>,
    pub(crate) receiver: mpsc::UnboundedReceiver<GuiEvent>,
}

impl Drop for GuiEventSubscription {
    fn drop(&mut self) {
        let mut mailboxes = crate::event_bus::EVENT_BUS
            .gui_event
            .mailboxes
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if mailboxes
            .get(&(self.scope, self.target))
            .is_some_and(|sender| sender.same_channel(&self.sender))
        {
            mailboxes.remove(&(self.scope, self.target));
        }
    }
}
