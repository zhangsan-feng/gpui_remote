use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionProfile {
    pub id: String,
    pub protocol: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub proxy: Option<ProxyConfig>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSession {
    pub id: String,
    pub profile_id: String,
}

impl WorkspaceSession {
    pub fn new(profile_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            profile_id: profile_id.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewSession {
    pub protocol: String,
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

#[cfg(test)]
mod tests {
    use super::NewSession;

    fn valid_session() -> NewSession {
        NewSession {
            protocol: "SSH".into(),
            name: "server".into(),
            host: "127.0.0.1".into(),
            port: 22,
            username: "root".into(),
            password: String::new(),
            proxy: None,
        }
    }

    #[test]
    fn validates_required_connection_fields() {
        let mut session = valid_session();
        assert!(session.validate().is_ok());

        session.host.clear();
        assert_eq!(session.validate(), Err("请输入 SSH 主机地址"));

        session.host = "localhost".into();
        session.username.clear();
        assert_eq!(session.validate(), Err("请输入 SSH 用户名"));
    }
}
