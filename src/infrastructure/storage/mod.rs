mod derive;
mod known_hosts;
mod repository;

use gpui::Global;
use known_hosts::HostPubKey;
use repository::session_repository::SessionStorageRepository;
use russh::keys::ssh_key::PublicKey;

pub struct Storage {
    pub session: SessionStorageRepository,
    pub host_pub_key: HostPubKey,
}

impl Global for Storage {}

impl Storage {
    pub fn new() -> Self {
        Self {
            session: SessionStorageRepository::new().expect("sqlite init failed"),
            host_pub_key: HostPubKey {},
        }
    }
}

pub(crate) fn verify_host_key(endpoint: &str, public_key: &PublicKey) -> anyhow::Result<bool> {
    HostPubKey {}.verify_or_remember(endpoint, public_key)
}
