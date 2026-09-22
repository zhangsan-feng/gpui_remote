use std::{fmt, path::PathBuf, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Ssh,
    Sftp,
}

impl Protocol {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Sftp => "SFTP",
        }
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Protocol {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_uppercase().as_str() {
            "SSH" => Ok(Self::Ssh),
            "SFTP" => Ok(Self::Sftp),
            protocol => Err(format!("不支持的连接协议: {protocol}")),
        }
    }
}

/// A saved connection profile groups the protocols that share one set of
/// connection credentials. SSH and SFTP intentionally belong to one profile;
/// the workspace `Protocol` above describes which view was opened from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionProtocol {
    SshAndSftp,
    Mysql,
    Pgsql,
    Redis,
}

impl ConnectionProtocol {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SshAndSftp => "SSH",
            Self::Mysql => "mysql",
            Self::Pgsql => "pgsql",
            Self::Redis => "redis",
        }
    }

    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::SshAndSftp => "SSH_AND_SFTP",
            Self::Mysql => "MYSQL",
            Self::Pgsql => "PGSQL",
            Self::Redis => "REDIS",
        }
    }

    pub const fn supports(self, protocol: Protocol) -> bool {
        match self {
            Self::SshAndSftp => matches!(protocol, Protocol::Ssh | Protocol::Sftp),
            Self::Mysql | Self::Pgsql | Self::Redis => false,
        }
    }
}

impl fmt::Display for ConnectionProtocol {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ConnectionProtocol {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_uppercase().as_str() {
            // SSH and SFTP were previously persisted as separate profile
            // protocols. Read both legacy values as the combined profile.
            "SSH" | "SFTP" | "SSH+SFTP" | "SSH + SFTP" | "SSH_AND_SFTP" => Ok(Self::SshAndSftp),
            "MYSQL" => Ok(Self::Mysql),
            "PGSQL" | "POSTGRES" | "POSTGRESQL" => Ok(Self::Pgsql),
            "REDIS" => Ok(Self::Redis),
            protocol => Err(format!("不支持的连接协议: {protocol}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionProfile {
    pub id: String,
    pub connection_protocol: ConnectionProtocol,
    /// The protocol of the currently opened workspace view.
    pub protocol: Protocol,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub private_key_path: Option<String>,
    pub proxy: Option<ProxyConfig>,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SftpSessionState {
    pub local_path: Option<PathBuf>,
    pub remote_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSession {
    pub connection_protocol: ConnectionProtocol,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub private_key_path: Option<String>,
    pub proxy: Option<ProxyConfig>,
}

impl NewSession {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.host.trim().is_empty() {
            return Err("请输入 SSH 主机地址");
        }
        if self.username.trim().is_empty() {
            return Err("请输入 SSH 用户名");
        }
        if self.port == 0 {
            return Err("SSH 端口必须大于 0");
        }
        if self.proxy.as_ref().is_some_and(|proxy| proxy.port == 0) {
            return Err("代理端口必须大于 0");
        }
        Ok(())
    }
}
