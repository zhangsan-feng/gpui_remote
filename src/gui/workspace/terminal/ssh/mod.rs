mod core;
mod runtime;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::infrastructure::storage::verify_host_key;

pub(super) use core::run_ssh_session;

const DEFAULT_COLUMNS: u32 = 120;
const DEFAULT_ROWS: u32 = 36;

trait SshStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> SshStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

struct ClientHandler {
    endpoint: String,
}

impl russh::client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match verify_host_key(&self.endpoint, server_public_key) {
            Ok(accepted) => {
                if !accepted {
                    log::info!("SSH host key changed for {}", self.endpoint);
                }
                Ok(accepted)
            }
            Err(error) => {
                log::info!("SSH host key verification failed: {error:#}");
                Ok(false)
            }
        }
    }
}
