use anyhow::{Context as _, Result};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use crate::{
    domain::session::{NewSession, ProxyConfig, SessionProfile},
    infrastructure::storage::derive::sqlite_drive::SqliteDrive,
};

pub struct SessionStorageRepository {
    drive: SqliteDrive,
}

impl SessionStorageRepository {
    pub fn new() -> Result<Self> {
        Ok(Self {
            drive: SqliteDrive::new()?,
        })
    }

    pub fn list(&self) -> Result<Vec<SessionProfile>> {
        let mut statement = self.drive.connection.prepare(
            "SELECT id, protocol, name, host, port, username, password,
                    proxy_host, proxy_port, proxy_username, proxy_password, created_at
             FROM sessions ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            let proxy_host: Option<String> = row.get(7)?;
            let proxy = if let Some(host) = proxy_host {
                Some(ProxyConfig {
                    host,
                    port: row.get::<_, Option<u16>>(8)?.unwrap_or(1080),
                    username: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
                    password: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
                })
            } else {
                None
            };
            Ok(SessionProfile {
                id: row.get(0)?,
                protocol: row.get(1)?,
                name: row.get(2)?,
                host: row.get(3)?,
                port: row.get(4)?,
                username: row.get(5)?,
                password: row.get(6)?,
                proxy,
                created_at: row.get(11)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("read sessions from SQLite")
    }

    pub fn insert(&self, draft: NewSession) -> Result<SessionProfile> {
        let profile = SessionProfile {
            id: Uuid::new_v4().to_string(),
            protocol: draft.protocol,
            name: draft.name.trim().to_owned(),
            host: draft.host.trim().to_owned(),
            port: draft.port,
            username: draft.username.trim().to_owned(),
            password: draft.password,
            proxy: draft.proxy,
            created_at: Utc::now().to_rfc3339(),
        };
        let proxy = profile.proxy.as_ref();
        self.drive.connection.execute(
            "INSERT INTO sessions (
                id, protocol, name, host, port, username, password,
                proxy_host, proxy_port, proxy_username, proxy_password, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                profile.id,
                profile.protocol,
                profile.name,
                profile.host,
                profile.port,
                profile.username,
                profile.password,
                proxy.map(|value| value.host.as_str()),
                proxy.map(|value| value.port),
                proxy.map(|value| value.username.as_str()),
                proxy.map(|value| value.password.as_str()),
                profile.created_at,
            ],
        )?;
        Ok(profile)
    }

    pub fn update(&self, id: &str, draft: NewSession) -> Result<SessionProfile> {
        let created_at: String = self
            .drive
            .connection
            .query_row(
                "SELECT created_at FROM sessions WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .context("find session to update")?;
        let profile = SessionProfile {
            id: id.to_owned(),
            protocol: draft.protocol,
            name: draft.name.trim().to_owned(),
            host: draft.host.trim().to_owned(),
            port: draft.port,
            username: draft.username.trim().to_owned(),
            password: draft.password,
            proxy: draft.proxy,
            created_at,
        };
        let proxy = profile.proxy.as_ref();
        self.drive.connection.execute(
            "UPDATE sessions SET
                protocol = ?2, name = ?3, host = ?4, port = ?5,
                username = ?6, password = ?7, proxy_host = ?8,
                proxy_port = ?9, proxy_username = ?10, proxy_password = ?11
             WHERE id = ?1",
            params![
                profile.id,
                profile.protocol,
                profile.name,
                profile.host,
                profile.port,
                profile.username,
                profile.password,
                proxy.map(|value| value.host.as_str()),
                proxy.map(|value| value.port),
                proxy.map(|value| value.username.as_str()),
                proxy.map(|value| value.password.as_str()),
            ],
        )?;
        Ok(profile)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.drive
            .connection
            .execute("DELETE FROM sessions WHERE id = ?1", [id])
            .context("delete session from SQLite")?;
        Ok(())
    }
}
