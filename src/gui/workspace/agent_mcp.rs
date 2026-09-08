use gpui_kit::*;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    application::agent_mcp::{
        AgentMcpCommand, AgentMcpReceiver, AgentSftpCommand, AgentSshCommand, ProfileSummary,
        TerminalReadPage, TerminalSummary,
    },
    domain::{
        session::Protocol,
        terminal::{TerminalHistoryPage, TerminalSessionCommand, TerminalStatus},
    },
    global_state::{GlobalEvent, read_global_state},
    infrastructure::storage::Storage,
};

use super::{Workspace, ssh::encode_agent_key};

const DEFAULT_READ_LIMIT: usize = 200;
const MAX_READ_LIMIT: usize = 2_000;

impl Workspace {
    pub(super) fn start_agent_mcp(&self, mut receiver: AgentMcpReceiver, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            while let Some(command) = receiver.recv().await {
                match command {
                    AgentMcpCommand::Ssh(AgentSshCommand::ReadTerminal {
                        workspace_id,
                        offset,
                        limit,
                        reply,
                    }) => {
                        let request = this.update(cx, |this, cx| {
                            this.prepare_terminal_read(workspace_id, offset, limit, cx)
                        });
                        let result = match request {
                            Ok(Ok((workspace_id, response))) => response
                                .await
                                .map(|page| map_terminal_page(workspace_id, page))
                                .map_err(|_| "终端读取请求已取消".to_owned()),
                            Ok(Err(error)) => Err(error),
                            Err(_) => Err("工作区已关闭".to_owned()),
                        };
                        let _ = reply.send(result);
                    }
                    command => {
                        if this
                            .update(cx, |this, cx| this.handle_agent_command(command, cx))
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        })
        .detach();
    }

    fn handle_agent_command(&mut self, command: AgentMcpCommand, cx: &mut Context<Self>) {
        match command {
            AgentMcpCommand::ListProfiles { reply } => {
                let result = cx
                    .global::<Storage>()
                    .session
                    .list()
                    .map(|profiles| {
                        profiles
                            .into_iter()
                            .map(|profile| ProfileSummary {
                                id: profile.id,
                                title: profile.name,
                                host: profile.host,
                            })
                            .collect()
                    })
                    .map_err(|error| format!("读取连接配置失败: {error:#}"));
                let _ = reply.send(result);
            }
            AgentMcpCommand::Ssh(AgentSshCommand::Open {
                profile_id,
                ip,
                title,
                reply,
            }) => {
                let result = self.open_agent_session(profile_id, Protocol::Ssh, ip, title, cx);
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::Open {
                profile_id,
                ip,
                title,
                reply,
            }) => {
                let result = self.open_agent_session(profile_id, Protocol::Sftp, ip, title, cx);
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::ListLocal { reply }) => {
                let result = Ok(self.sftp.read(cx).mcp_local_directory());
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::ChangeLocalDirectory {
                workspace_id,
                ip,
                title,
                path,
                reply,
            }) => {
                let result = self
                    .sftp
                    .update(cx, |sftp, cx| {
                        sftp.mcp_change_local_directory(workspace_id, ip, title, path, cx)
                    })
                    .map_err(|_| "工作区已关闭".to_owned());
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::ListRemote {
                workspace_id,
                reply,
            }) => {
                let result = self.sftp.read(cx).mcp_remote_directory(&workspace_id);
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::ChangeRemoteDirectory {
                workspace_id,
                ip,
                title,
                path,
                reply,
            }) => {
                let result = self
                    .sftp
                    .update(cx, |sftp, cx| {
                        sftp.mcp_change_remote_directory(workspace_id, ip, title, path, cx)
                    })
                    .map_err(|_| "工作区已关闭".to_owned());
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::Upload {
                workspace_id,
                local_paths,
                reply,
            }) => {
                let result = self
                    .sftp
                    .update(cx, |sftp, cx| {
                        sftp.mcp_upload(&workspace_id, local_paths, cx)
                    })
                    .map_err(|_| "工作区已关闭".to_owned());
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::Download {
                workspace_id,
                remote_paths,
                reply,
            }) => {
                let result = self
                    .sftp
                    .update(cx, |sftp, cx| {
                        sftp.mcp_download(&workspace_id, remote_paths, cx)
                    })
                    .map_err(|_| "工作区已关闭".to_owned());
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::ListTransfers {
                workspace_id,
                reply,
            }) => {
                let result = self.sftp.read(cx).mcp_transfers(&workspace_id);
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::WatchLocal {
                workspace_id,
                ip,
                title,
                local_path,
                reply,
            }) => {
                let result = self
                    .sftp
                    .update(cx, |sftp, cx| {
                        sftp.mcp_watch_local(workspace_id, ip, title, local_path, cx)
                    })
                    .map_err(|_| "工作区已关闭".to_owned());
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::StopWatchingLocal {
                workspace_id,
                ip,
                title,
                local_path,
                reply,
            }) => {
                let result = self
                    .sftp
                    .update(cx, |sftp, cx| {
                        sftp.mcp_stop_watching_local(workspace_id, ip, title, local_path, cx)
                    })
                    .map_err(|_| "工作区已关闭".to_owned());
                let _ = reply.send(result);
            }
            AgentMcpCommand::Sftp(AgentSftpCommand::ListLocalWatches {
                workspace_id,
                ip,
                title,
                reply,
            }) => {
                let result = self
                    .sftp
                    .read(cx)
                    .mcp_list_local_watches(&workspace_id, &ip, &title);
                let _ = reply.send(result);
            }
            AgentMcpCommand::Ssh(AgentSshCommand::ListTerminals { reply }) => {
                let selected_id = self.workspace.read(cx).selected_id();
                let terminals = self
                    .workspace
                    .read(cx)
                    .sessions()
                    .iter()
                    .map(|opened| {
                        let status = self
                            .terminal
                            .read(cx)
                            .model(&opened.id)
                            .map(|model| terminal_status_name(&model.read().status))
                            .unwrap_or("connecting");
                        TerminalSummary {
                            workspace_id: opened.id.clone(),
                            profile_id: opened.profile.id.clone(),
                            ip: opened.profile.host.clone(),
                            title: opened.profile.name.clone(),
                            host: opened.profile.host.clone(),
                            status: status.to_owned(),
                            selected: selected_id == Some(opened.id.as_str()),
                        }
                    })
                    .collect();
                let _ = reply.send(Ok(terminals));
            }
            AgentMcpCommand::Ssh(AgentSshCommand::SelectTerminal {
                workspace_id,
                ip,
                title,
                reply,
            }) => {
                let session_identity = self
                    .workspace
                    .read(cx)
                    .sessions()
                    .iter()
                    .find(|opened| opened.id == workspace_id)
                    .map(|opened| (opened.profile.host.clone(), opened.profile.name.clone()));
                let result = match session_identity {
                    None => Err(format!("终端会话不存在: {workspace_id}")),
                    Some((actual_ip, actual_title)) if actual_ip == ip && actual_title == title => {
                        self.workspace.update(cx, |workspace, cx| {
                            workspace.activate(&workspace_id, cx);
                        });
                        Ok(())
                    }
                    Some(_) => Err(format!(
                        "终端会话信息不匹配: {workspace_id}，请确认 ip 和 title"
                    )),
                };
                let _ = reply.send(result);
            }
            AgentMcpCommand::Ssh(AgentSshCommand::SendText {
                workspace_id,
                text,
                reply,
            }) => {
                let result = self
                    .resolve_terminal_id(workspace_id, cx)
                    .map(|workspace_id| {
                        self.terminal
                            .read(cx)
                            .send_input(&workspace_id, text.into_bytes());
                    });
                let _ = reply.send(result);
            }
            AgentMcpCommand::Ssh(AgentSshCommand::SendKey {
                workspace_id,
                key,
                control,
                alt,
                shift,
                reply,
            }) => {
                let result = self
                    .resolve_terminal_id(workspace_id, cx)
                    .and_then(|workspace_id| {
                        let terminal = self.terminal.read(cx);
                        let application_cursor = terminal
                            .model(&workspace_id)
                            .is_some_and(|model| model.read().frame.application_cursor);
                        let input = encode_agent_key(&key, control, alt, shift, application_cursor)
                            .ok_or_else(|| format!("不支持的终端按键: {key}"))?;
                        terminal.send_input(&workspace_id, input);
                        Ok(())
                    });
                let _ = reply.send(result);
            }
            AgentMcpCommand::Ssh(AgentSshCommand::ReadTerminal { .. }) => unreachable!(),
        }
    }

    fn open_agent_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
        cx: &mut Context<Self>,
    ) -> Result<String, String> {
        cx.global::<Storage>()
            .session
            .find(&profile_id)
            .map_err(|error| format!("读取连接配置失败: {error:#}"))
            .and_then(|profile| profile.ok_or_else(|| format!("连接配置不存在: {profile_id}")))
            .and_then(|mut profile| {
                if profile.host != ip || profile.name != title {
                    return Err(format!(
                        "连接配置与 ip/title 不匹配: {profile_id}，请确认 ip 和 title"
                    ));
                }
                profile.protocol = protocol;
                let workspace_id = Uuid::new_v4().to_string();
                read_global_state(cx).update(cx, |_, cx| {
                    cx.emit(GlobalEvent::OpenWorkspaceSession(
                        workspace_id.clone(),
                        profile,
                    ));
                });
                Ok(workspace_id)
            })
    }

    fn prepare_terminal_read(
        &self,
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
        cx: &App,
    ) -> Result<(String, oneshot::Receiver<TerminalHistoryPage>), String> {
        let workspace_id = self.resolve_terminal_id(workspace_id, cx)?;
        let commands = self
            .terminal
            .read(cx)
            .command_sender(&workspace_id)
            .ok_or_else(|| format!("终端会话不存在: {workspace_id}"))?;
        let (reply, response) = oneshot::channel();
        commands
            .send(TerminalSessionCommand::Read {
                offset,
                limit: normalize_read_limit(limit),
                reply,
            })
            .map_err(|_| format!("终端会话不可用: {workspace_id}"))?;
        Ok((workspace_id, response))
    }

    fn resolve_terminal_id(
        &self,
        workspace_id: Option<String>,
        cx: &App,
    ) -> Result<String, String> {
        let workspace_id = workspace_id
            .or_else(|| self.workspace.read(cx).selected_id().map(str::to_owned))
            .ok_or_else(|| "当前没有选中的终端会话".to_owned())?;
        self.terminal
            .read(cx)
            .model(&workspace_id)
            .map(|_| workspace_id.clone())
            .ok_or_else(|| format!("终端会话不存在: {workspace_id}"))
    }
}

fn normalize_read_limit(limit: usize) -> usize {
    if limit == 0 {
        DEFAULT_READ_LIMIT
    } else {
        limit.min(MAX_READ_LIMIT)
    }
}

fn map_terminal_page(workspace_id: String, page: TerminalHistoryPage) -> TerminalReadPage {
    TerminalReadPage {
        workspace_id,
        text: page.text,
        total_lines: page.total_lines,
        offset: page.offset,
        limit: page.limit,
        has_more: page.has_more,
    }
}

fn terminal_status_name(status: &TerminalStatus) -> &'static str {
    match status {
        TerminalStatus::Connecting => "connecting",
        TerminalStatus::Connected => "connected",
        TerminalStatus::Disconnected => "disconnected",
        TerminalStatus::Failed => "failed",
    }
}
