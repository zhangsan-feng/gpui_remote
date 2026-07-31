use std::{fmt, str::FromStr};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionProfile {
    pub id: String,
    pub protocol: Protocol,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub proxy: Option<ProxyConfig>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSession {
    pub protocol: Protocol,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
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


