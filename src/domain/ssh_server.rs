use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SshServerSettings {
    pub(crate) enabled: bool,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) password: String,
}

impl Default for SshServerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "0.0.0.0".to_owned(),
            port: 2222,
            username: std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .unwrap_or_default(),
            password: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SshServerStatus {
    pub(crate) running: bool,
    pub(crate) address: Option<String>,
    pub(crate) error: Option<String>,
}
