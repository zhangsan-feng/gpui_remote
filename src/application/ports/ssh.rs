use std::{future::Future, pin::Pin};

use anyhow::Result;

pub(crate) type SshOpenFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn SshShell>>> + Send + 'a>>;
pub(crate) type SshWaitFuture<'a> =
    Pin<Box<dyn Future<Output = Option<SshChannelEvent>> + Send + 'a>>;
pub(crate) type SshResultFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

pub(crate) trait SshShell: Send + Sync {
    fn wait_event(&self) -> SshWaitFuture<'_>;
    fn send_data(&self, data: Vec<u8>) -> SshResultFuture<'_>;
    fn resize(&self, columns: u32, rows: u32) -> SshResultFuture<'_>;
    fn close_channel(&self) -> SshResultFuture<'_>;
    fn disconnect(&self) -> SshResultFuture<'_>;
}

pub(crate) enum SshChannelEvent {
    Data(Vec<u8>),
    ExitStatus(u32),
    ExitSignal {
        signal_name: String,
        core_dumped: bool,
        error_message: String,
    },
    Eof,
    Close,
    Other,
}
