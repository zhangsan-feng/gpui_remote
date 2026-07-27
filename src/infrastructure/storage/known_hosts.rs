use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context as _, Result};
use russh::keys::ssh_key::{HashAlg, PublicKey};

const KNOWN_HOSTS_PATH: &str = "data/known_hosts.json";

pub struct HostPubKey {}

impl HostPubKey {
    pub fn verify_or_remember(&self, endpoint: &str, public_key: &PublicKey) -> Result<bool> {
        let path = Path::new(KNOWN_HOSTS_PATH);
        let mut known_hosts = if path.exists() {
            serde_json::from_slice::<BTreeMap<String, String>>(
                &fs::read(path).context("读取 known_hosts 失败")?,
            )
            .context("解析 known_hosts 失败")?
        } else {
            BTreeMap::new()
        };
        let fingerprint = public_key.fingerprint(HashAlg::Sha256).to_string();
        if let Some(expected) = known_hosts.get(endpoint) {
            return Ok(expected == &fingerprint);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("创建 known_hosts 目录失败")?;
        }
        known_hosts.insert(endpoint.to_owned(), fingerprint);
        fs::write(
            path,
            serde_json::to_vec_pretty(&known_hosts).context("序列化 known_hosts 失败")?,
        )
        .context("保存 known_hosts 失败")?;
        Ok(true)
    }
}
