use std::time::{Duration, Instant};

use crate::{
    application::ApplicationResult,
    domain::terminal::{TerminalFrame, TerminalStatus},
};

use super::SshApplication;

#[derive(Clone, Copy, Debug)]
enum RemoteShell {
    Cmd,
    PowerShell,
    Posix,
}

impl SshApplication {
    pub(crate) async fn set_tunnel_proxy_environment(
        &self,
        workspace_id: &str,
        remote_port: u16,
    ) -> ApplicationResult<()> {
        let started = Instant::now();
        let shell = loop {
            let snapshot = self.terminal_snapshot(workspace_id)?;
            if matches!(
                snapshot.status,
                TerminalStatus::Disconnected | TerminalStatus::Failed
            ) {
                return Err(snapshot
                    .message
                    .unwrap_or_else(|| "SSH 连接已断开".to_owned()));
            }
            if let Some(shell) = detect_shell(&snapshot.frame) {
                break shell;
            }
            if started.elapsed() >= Duration::from_secs(10) {
                // Preserve the existing Unix behavior for customized, unrecognized prompts.
                log::warn!(
                    "ssh_tunnel_shell_detection_timeout workspace_id={workspace_id} fallback=posix"
                );
                break RemoteShell::Posix;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        let proxy_url = format!("http://127.0.0.1:{remote_port}");
        let commands = match shell {
            RemoteShell::Cmd => {
                format!("set \"http_proxy={proxy_url}\"\rset \"https_proxy={proxy_url}\"\r")
            }
            RemoteShell::PowerShell => {
                format!("$env:http_proxy='{proxy_url}'; $env:https_proxy='{proxy_url}'\r")
            }
            RemoteShell::Posix => {
                format!("export http_proxy='{proxy_url}'\nexport https_proxy='{proxy_url}'\n")
            }
        };
        log::info!(
            "ssh_tunnel_proxy_environment_sending workspace_id={workspace_id} shell={shell:?} proxy={proxy_url}"
        );
        // Send to the existing interactive channel so variables remain in that shell.
        self.send_input(workspace_id, commands.into_bytes()).await
    }
}

fn detect_shell(frame: &TerminalFrame) -> Option<RemoteShell> {
    let prompt = frame.lines.iter().rev().find_map(|line| {
        let text: String = line.spans.iter().map(|span| span.text.as_str()).collect();
        let text = text.trim().to_owned();
        (!text.is_empty()).then_some(text)
    })?;
    if prompt.starts_with("PS ") && prompt.ends_with('>') {
        return Some(RemoteShell::PowerShell);
    }
    if prompt.ends_with('>')
        && ((prompt.as_bytes().get(1) == Some(&b':') && prompt.as_bytes()[0].is_ascii_alphabetic())
            || prompt.starts_with("\\\\"))
    {
        return Some(RemoteShell::Cmd);
    }
    if prompt.ends_with(['$', '#', '%']) {
        return Some(RemoteShell::Posix);
    }
    None
}
