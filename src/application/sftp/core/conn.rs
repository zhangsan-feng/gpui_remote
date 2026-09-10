use std::time::Duration;

use crate::infrastructure::storage::verify_host_key;
use anyhow::Result;
use russh::client;

pub(super) struct SftpClientHandler {
    pub(super) endpoint: String,
}

impl client::Handler for SftpClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let endpoint = self.endpoint.clone();
        let public_key = server_public_key.clone();
        match tokio::task::spawn_blocking(move || verify_host_key(&endpoint, &public_key)).await {
            Ok(Ok(accepted)) => Ok(accepted),
            Ok(Err(error)) => {
                log::info!("SFTP host key verification failed: {error:#}");
                Ok(false)
            }
            Err(error) => {
                log::info!("SFTP host key verification task failed: {error}");
                Ok(false)
            }
        }
    }
}

pub(super) fn ssh_config() -> client::Config {
    client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        ..Default::default()
    }
}
