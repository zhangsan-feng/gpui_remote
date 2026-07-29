use std::{sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, client};
use tokio::sync::mpsc;

use crate::domain::terminal::{
    TerminalData, TerminalFrame, TerminalSessionCommand, TerminalStatus,
};

use super::super::{buffer::TerminalBuffer, pty::TerminalModel};

const REFRESH_INTERVAL: Duration = Duration::from_millis(33);

pub(super) async fn run_connected_terminal_session(
    mut reader: ChannelReadHalf,
    writer: ChannelWriteHalf<client::Msg>,
    command_tx: mpsc::UnboundedSender<TerminalSessionCommand>,
    mut commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
    model: Arc<TerminalModel>,
) -> Result<Option<String>> {
    let mut buffer = TerminalBuffer::new(command_tx);
    let mut refresh = tokio::time::interval(REFRESH_INTERVAL);
    refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    refresh.tick().await;

    let (mut last_frame, status, message) = {
        let data = model.read();
        (
            data.frame.clone(),
            data.status.clone(),
            data.message.clone(),
        )
    };
    let mut dirty = false;
    let mut exit_message = None;

    loop {
        tokio::select! {
            biased;
            command = commands.recv() => {
                if !apply_command(command, &mut buffer, &writer, &mut dirty).await? {
                    break;
                }
            }
            _ = refresh.tick() => {
                if dirty {
                    publish_model(
                        &mut buffer,
                        &model,
                        &mut last_frame,
                        &status,
                        &message,
                    );
                    dirty = false;
                }
            }
            message = reader.wait() => match message {
                Some(ChannelMsg::Data { data })
                | Some(ChannelMsg::ExtendedData { data, .. }) => {
                    buffer.process(&data);
                    dirty = true;
                }
                Some(ChannelMsg::ExitStatus { exit_status }) => {
                    exit_message = Some(format!(
                        "远程 Shell 已退出（状态码 {exit_status}）"
                    ));
                }
                Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                _ => {}
            }
        }
    }

    if dirty {
        publish_model(&mut buffer, &model, &mut last_frame, &status, &message);
    }
    Ok(exit_message)
}

async fn apply_command(
    command: Option<TerminalSessionCommand>,
    buffer: &mut TerminalBuffer,
    writer: &ChannelWriteHalf<client::Msg>,
    dirty: &mut bool,
) -> Result<bool> {
    match command {
        Some(TerminalSessionCommand::Input(data)) => {
            writer.data_bytes(data).await.context("发送终端输入失败")?;
        }
        Some(TerminalSessionCommand::Resize { columns, rows }) => {
            buffer.resize(columns, rows);
            writer
                .window_change(columns.max(1), rows.max(1), 0, 0)
                .await
                .context("调整 PTY 大小失败")?;
            *dirty = true;
        }
        Some(TerminalSessionCommand::Scroll { lines }) => {
            buffer.scroll(lines);
            *dirty = true;
        }
        Some(TerminalSessionCommand::ScrollTo { offset }) => {
            buffer.scroll_to(offset);
            *dirty = true;
        }
        Some(TerminalSessionCommand::Read {
            offset,
            limit,
            reply,
        }) => {
            let _ = reply.send(buffer.read_text(offset, limit));
        }
        Some(TerminalSessionCommand::Disconnect) | None => {
            return Ok(false);
        }
    }
    Ok(true)
}

fn publish_model(
    buffer: &mut TerminalBuffer,
    model: &TerminalModel,
    last_frame: &mut Arc<TerminalFrame>,
    status: &TerminalStatus,
    message: &Option<String>,
) {
    let next_frame = Arc::new(buffer.frame_reusing(Some(last_frame.as_ref())));
    if same_frame(last_frame, &next_frame) {
        return;
    }
    *last_frame = next_frame;
    model.replace(TerminalData {
        frame: last_frame.clone(),
        status: status.clone(),
        message: message.clone(),
    });
}

fn same_frame(left: &TerminalFrame, right: &TerminalFrame) -> bool {
    left.application_cursor == right.application_cursor
        && left.history_size == right.history_size
        && left.display_offset == right.display_offset
        && left.lines.len() == right.lines.len()
        && left
            .lines
            .iter()
            .zip(right.lines.iter())
            .all(|(left, right)| Arc::ptr_eq(left, right))
}
