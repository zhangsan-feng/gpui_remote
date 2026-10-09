use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::data_context::DATA_CONTEXT;
use crate::domain::{
    session::{Protocol, SessionProfile},
    terminal::{TerminalData, TerminalSessionCommand, TerminalStatus},
};

use super::key::{encode_control_key, encode_special_key};
use super::{TerminalRuntime, disconnect, new_runtime};

#[derive(Clone)]
pub struct SshApplication {
    inner: Arc<SshApplicationInner>,
    pub(crate) sessions: crate::application::session::SessionApplication,
}

struct SshApplicationInner {
    runtimes: RwLock<HashMap<String, TerminalRuntime>>,
}

impl SshApplication {
    pub(crate) fn new(sessions: crate::application::session::SessionApplication) -> Self {
        Self {
            inner: Arc::new(SshApplicationInner {
                runtimes: RwLock::new(HashMap::new()),
            }),
            sessions,
        }
    }

    pub async fn open(&self, workspace_id: String, profile: SessionProfile) -> Result<(), String> {
        if profile.protocol != Protocol::Ssh {
            return Err(format!("SSH 模块不支持 {} 协议", profile.protocol));
        }
        self.close_if_present(&workspace_id);
        let runtime = new_runtime(workspace_id.clone(), profile);
        self.inner
            .runtimes
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(workspace_id, runtime);
        Ok(())
    }

    pub async fn close(&self, workspace_id: &str) -> Result<(), String> {
        let runtime = self
            .inner
            .runtimes
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id)
            .ok_or_else(|| format!("SSH 会话不存在: {workspace_id}"))?;
        disconnect(runtime);
        Ok(())
    }

    pub async fn send_input(&self, workspace_id: &str, input: Vec<u8>) -> Result<(), String> {
        let commands = self.live_commands(workspace_id, "send_input")?;
        commands
            .send(TerminalSessionCommand::Input(input))
            .map_err(|_| Self::command_unavailable(workspace_id, "send_input"))
    }

    pub async fn resize(&self, workspace_id: &str, columns: u32, rows: u32) -> Result<(), String> {
        let commands = self.commands(workspace_id)?;
        commands
            .send(TerminalSessionCommand::Resize { columns, rows })
            .map_err(|_| Self::command_unavailable(workspace_id, "resize"))
    }

    pub async fn scroll(&self, workspace_id: &str, lines: i32) -> Result<(), String> {
        let commands = self.commands(workspace_id)?;
        commands
            .send(TerminalSessionCommand::Scroll { lines })
            .map_err(|_| Self::command_unavailable(workspace_id, "scroll"))
    }

    pub async fn scroll_to(&self, workspace_id: &str, offset: usize) -> Result<(), String> {
        let commands = self.commands(workspace_id)?;
        commands
            .send(TerminalSessionCommand::ScrollTo { offset })
            .map_err(|_| Self::command_unavailable(workspace_id, "scroll_to"))
    }

    pub async fn send_key(
        &self,
        workspace_id: &str,
        key: &str,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> Result<(), String> {
        let application_cursor = self
            .terminal_snapshot(workspace_id)?
            .frame
            .application_cursor;
        let normalized_key = key.to_ascii_lowercase();
        let input = if let Some(sequence) = encode_special_key(&normalized_key, application_cursor)
        {
            let mut bytes = Vec::with_capacity(sequence.len() + usize::from(alt));
            if alt {
                bytes.push(0x1b);
            }
            bytes.extend_from_slice(sequence.as_bytes());
            bytes
        } else if control {
            vec![encode_control_key(key).ok_or_else(|| format!("不支持的终端按键: {key}"))?]
        } else {
            let text = if shift && key.chars().count() == 1 {
                key.to_uppercase()
            } else {
                key.to_owned()
            };
            let mut bytes = Vec::with_capacity(text.len() + usize::from(alt));
            if alt {
                bytes.push(0x1b);
            }
            bytes.extend_from_slice(text.as_bytes());
            bytes
        };
        self.send_input(workspace_id, input).await
    }

    pub fn terminal_snapshot(&self, workspace_id: &str) -> Result<TerminalData, String> {
        DATA_CONTEXT
            .terminal_snapshot(workspace_id)
            .map(|snapshot| snapshot.data.as_ref().clone())
            .ok_or_else(|| format!("SSH 会话不存在: {workspace_id}"))
    }

    pub(crate) fn runtime_available(&self, workspace_id: &str) -> bool {
        self.inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .is_some_and(|runtime| {
                let Some(snapshot) = runtime.model.read() else {
                    return false;
                };
                let status = snapshot.data.status.clone();
                matches!(
                    status,
                    TerminalStatus::Connecting | TerminalStatus::Connected
                )
            })
    }

    fn live_commands(
        &self,
        workspace_id: &str,
        operation: &str,
    ) -> Result<tokio::sync::mpsc::UnboundedSender<TerminalSessionCommand>, String> {
        let commands = self.commands(workspace_id)?;
        let status = self
            .inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .and_then(|runtime| {
                runtime
                    .model
                    .read()
                    .map(|snapshot| snapshot.data.status.clone())
            });
        if matches!(
            status,
            Some(TerminalStatus::Disconnected | TerminalStatus::Failed)
        ) {
            return Err(Self::command_unavailable(workspace_id, operation));
        }
        Ok(commands)
    }

    fn commands(
        &self,
        workspace_id: &str,
    ) -> Result<tokio::sync::mpsc::UnboundedSender<TerminalSessionCommand>, String> {
        let commands = self
            .inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .map(|runtime| runtime.commands.clone())
            .ok_or_else(|| format!("SSH 会话不存在: {workspace_id}"))?;
        if commands.is_closed() {
            log::warn!(
                "SSH runtime command channel is closed: workspace_id={workspace_id}, reason=runtime_stopped"
            );
        }
        Ok(commands)
    }

    fn command_unavailable(workspace_id: &str, operation: &str) -> String {
        log::warn!(
            "SSH command rejected: workspace_id={workspace_id}, operation={operation}, reason=runtime_command_channel_closed"
        );
        format!("SSH 会话不可用: {workspace_id}")
    }

    fn close_if_present(&self, workspace_id: &str) {
        if let Some(runtime) = self
            .inner
            .runtimes
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id)
        {
            disconnect(runtime);
        }
    }
}

impl Drop for SshApplicationInner {
    fn drop(&mut self) {
        let runtimes = std::mem::take(
            &mut *self
                .runtimes
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        for runtime in runtimes.into_values() {
            disconnect(runtime);
        }
    }
}
