mod derive;
mod known_hosts;
mod repository;

use known_hosts::HostPubKey;
use russh::keys::ssh_key::PublicKey;

pub(crate) use repository::session_repository::SessionStorageRepository;

pub(crate) struct Storage {
    pub(crate) session: SessionStorageRepository,
    pub(crate) host_pub_key: HostPubKey,
}

impl Storage {
    pub(crate) fn new() -> Self {
        Self {
            session: SessionStorageRepository::new().expect("sqlite init failed"),
            host_pub_key: HostPubKey {},
        }
    }
}

pub(crate) fn verify_host_key(endpoint: &str, public_key: &PublicKey) -> anyhow::Result<bool> {
    HostPubKey {}.verify_or_update(endpoint, public_key)
}
