use std::{
    io::{Read, Write},
    time::Duration,
};

use alacritty_terminal::{
    event::{OnResize, WindowSize},
    tty::{self, ChildEvent, EventedPty, EventedReadWrite, Options, Shell},
};
use russh::{ChannelId, server::Handle};
use tokio::sync::{mpsc, oneshot};

pub(super) enum ShellCommand {
    Input(Vec<u8>),
    Resize(WindowSize),
    Stop,
}

enum ShellOutput {
    Data(Vec<u8>),
    Exit(u32),
}

pub(super) fn window_size(
    columns: u32,
    rows: u32,
    pixel_width: u32,
    pixel_height: u32,
) -> WindowSize {
    let columns = columns.clamp(1, u16::MAX as u32) as u16;
    let rows = rows.clamp(1, u16::MAX as u32) as u16;
    WindowSize {
        num_lines: rows,
        num_cols: columns,
        cell_width: (pixel_width / columns as u32).clamp(1, u16::MAX as u32) as u16,
        cell_height: (pixel_height / rows as u32).clamp(1, u16::MAX as u32) as u16,
    }
}

pub(super) async fn spawn_shell(
    size: WindowSize,
    handle: Handle,
    channel: ChannelId,
) -> Result<mpsc::Sender<ShellCommand>, String> {
    let (commands, command_rx) = mpsc::channel(128);
    let (output_tx, mut output_rx) = mpsc::channel(64);
    let (ready_tx, ready_rx) = oneshot::channel();
    tokio::task::spawn_blocking(move || run_shell(size, command_rx, output_tx, ready_tx));
    ready_rx
        .await
        .map_err(|_| "启动本机 Shell 任务中断".to_owned())??;
    tokio::spawn(async move {
        while let Some(output) = output_rx.recv().await {
            match output {
                ShellOutput::Data(data) => {
                    if handle.data(channel, data).await.is_err() {
                        break;
                    }
                }
                ShellOutput::Exit(code) => {
                    let _ = handle.exit_status_request(channel, code).await;
                    let _ = handle.eof(channel).await;
                    let _ = handle.close(channel).await;
                    break;
                }
            }
        }
    });
    Ok(commands)
}

fn run_shell(
    size: WindowSize,
    mut commands: mpsc::Receiver<ShellCommand>,
    output: mpsc::Sender<ShellOutput>,
    ready: oneshot::Sender<Result<(), String>>,
) {
    #[cfg(windows)]
    let program = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_owned());
    #[cfg(not(windows))]
    let program = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    let options = Options {
        shell: Some(Shell::new(program, Vec::new())),
        ..Default::default()
    };
    let mut pty = match tty::new(&options, size, 0) {
        Ok(pty) => {
            let _ = ready.send(Ok(()));
            pty
        }
        Err(error) => {
            let _ = ready.send(Err(format!("启动本机 Shell 失败: {error}")));
            return;
        }
    };
    let mut pending = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        while let Ok(command) = commands.try_recv() {
            match command {
                ShellCommand::Input(bytes) => {
                    pending.extend(bytes);
                    if pending.len() > 1024 * 1024 {
                        log::warn!("ssh_shell_input_overflow");
                        return;
                    }
                }
                ShellCommand::Resize(size) => pty.on_resize(size),
                ShellCommand::Stop => return,
            }
        }
        if commands.is_closed() {
            return;
        }
        if !pending.is_empty() {
            match pty.writer().write(&pending) {
                Ok(written) => {
                    pending.drain(..written);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => {
                    log::warn!("ssh_shell_write_failed: {error}");
                    return;
                }
            }
        }
        match pty.reader().read(&mut buffer) {
            Ok(read) if read > 0 => {
                if output
                    .blocking_send(ShellOutput::Data(buffer[..read].to_vec()))
                    .is_err()
                {
                    return;
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => {
                log::warn!("ssh_shell_read_failed: {error}");
                return;
            }
        }
        if let Some(ChildEvent::Exited(status)) = pty.next_child_event() {
            let code = status.and_then(|status| status.code()).unwrap_or(0).max(0) as u32;
            let _ = output.blocking_send(ShellOutput::Exit(code));
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
