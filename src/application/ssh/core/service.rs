use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio::sync::{Notify, oneshot};

use crate::domain::{
    session::{Protocol, SessionProfile},
    terminal::{TerminalData, TerminalHistoryPage, TerminalSessionCommand},
};

use super::key::{encode_control_key, encode_special_key};
use super::{TerminalRuntime, disconnect, new_runtime};

#[derive(Clone)]
pub struct SshApplication {
    inner: Arc<SshApplicationInner>,
}

struct SshApplicationInner {
    runtimes: RwLock<HashMap<String, TerminalRuntime>>,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
}

impl Default for SshApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl SshApplication {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(SshApplicationInner {
                runtimes: RwLock::new(HashMap::new()),
                updates: Arc::new(Notify::new()),
                status_updates: Arc::new(Notify::new()),
            }),
        }
    }

    pub async fn open(&self, workspace_id: String, profile: SessionProfile) -> Result<(), String> {
        if profile.protocol != Protocol::Ssh {
            return Err(format!("SSH 模块不支持 {} 协议", profile.protocol));
        }
        self.close_if_present(&workspace_id);
        let runtime = new_runtime(
            profile,
            self.inner.updates.clone(),
            self.inner.status_updates.clone(),
        );
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
        let commands = self.commands(workspace_id)?;
        commands
            .send(TerminalSessionCommand::Input(input))
            .map_err(|_| format!("SSH 会话不可用: {workspace_id}"))
    }

    pub async fn resize(&self, workspace_id: &str, columns: u32, rows: u32) -> Result<(), String> {
        let commands = self.commands(workspace_id)?;
        commands
            .send(TerminalSessionCommand::Resize { columns, rows })
            .map_err(|_| format!("SSH 会话不可用: {workspace_id}"))
    }

    pub async fn scroll(&self, workspace_id: &str, lines: i32) -> Result<(), String> {
        let commands = self.commands(workspace_id)?;
        commands
            .send(TerminalSessionCommand::Scroll { lines })
            .map_err(|_| format!("SSH 会话不可用: {workspace_id}"))
    }

    pub async fn scroll_to(&self, workspace_id: &str, offset: usize) -> Result<(), String> {
        let commands = self.commands(workspace_id)?;
        commands
            .send(TerminalSessionCommand::ScrollTo { offset })
            .map_err(|_| format!("SSH 会话不可用: {workspace_id}"))
    }

    pub async fn send_key(
        &self,
        workspace_id: &str,
        key: &str,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> Result<(), String> {
        let application_cursor = self.snapshot(workspace_id)?.frame.application_cursor;
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

    pub async fn read(
        &self,
        workspace_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<TerminalHistoryPage, String> {
        let commands = self.commands(workspace_id)?;
        let (reply, response) = oneshot::channel();
        commands
            .send(TerminalSessionCommand::Read {
                offset,
                limit,
                reply,
            })
            .map_err(|_| format!("SSH 会话不可用: {workspace_id}"))?;
        response
            .await
            .map_err(|_| format!("SSH 历史读取已取消: {workspace_id}"))
    }

    pub fn snapshot(&self, workspace_id: &str) -> Result<TerminalData, String> {
        self.inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .map(|runtime| runtime.model.read().clone())
            .ok_or_else(|| format!("SSH 会话不存在: {workspace_id}"))
    }

    pub fn revision(&self, workspace_id: &str) -> Result<u64, String> {
        self.inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .map(|runtime| runtime.model.revision())
            .ok_or_else(|| format!("SSH 会话不存在: {workspace_id}"))
    }

    pub fn updates(&self) -> Arc<Notify> {
        self.inner.updates.clone()
    }

    pub fn status_updates(&self) -> Arc<Notify> {
        self.inner.status_updates.clone()
    }

    pub(crate) fn runtime_available(&self, workspace_id: &str) -> bool {
        self.inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .is_some_and(|runtime| !runtime.commands.is_closed())
    }

    fn commands(
        &self,
        workspace_id: &str,
    ) -> Result<tokio::sync::mpsc::UnboundedSender<TerminalSessionCommand>, String> {
        self.inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .map(|runtime| runtime.commands.clone())
            .ok_or_else(|| format!("SSH 会话不存在: {workspace_id}"))
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
