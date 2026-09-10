use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::{Context as _, Result};
use chrono::Utc;
use rusqlite::{OptionalExtension, Row, params, types::Type};
use uuid::Uuid;

use crate::{
    domain::session::{NewSession, Protocol, ProxyConfig, SessionProfile, SftpSessionState},
    infrastructure::storage::derive::sqlite_drive::SqliteDrive,
};

#[derive(Clone)]
pub struct SessionStorageRepository {
    drive: Arc<Mutex<SqliteDrive>>,
}

impl SessionStorageRepository {
    pub fn new() -> Result<Self> {
        let drive = SqliteDrive::new()?;
        Ok(Self {
            drive: Arc::new(Mutex::new(drive)),
        })
    }

    fn lock_drive(&self) -> MutexGuard<'_, SqliteDrive> {
        self.drive
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn list(&self) -> Result<Vec<SessionProfile>> {
        let drive = self.lock_drive();
        let mut statement = drive.connection.prepare(
            "SELECT id, protocol, name, host, port, username, password,
                    private_key_path, proxy_host, proxy_port, proxy_username,
                    proxy_password, created_at
             FROM sessions ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([], map_session)?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("read sessions from SQLite")
    }

    pub fn find(&self, id: &str) -> Result<Option<SessionProfile>> {
        let drive = self.lock_drive();
        drive
            .connection
            .query_row(
                "SELECT id, protocol, name, host, port, username, password,
                        private_key_path, proxy_host, proxy_port, proxy_username,
                        proxy_password, created_at
                 FROM sessions WHERE id = ?1",
                [id],
                map_session,
            )
            .optional()
            .context("find top_session in SQLite")
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
            private_key_path: normalize_private_key_path(draft.private_key_path),
            proxy: draft.proxy,
            created_at: Utc::now().to_rfc3339(),
        };
        let proxy = profile.proxy.as_ref();
        let drive = self.lock_drive();
        drive.connection.execute(
            "INSERT INTO sessions (
                id, protocol, name, host, port, username, password,
                private_key_path, proxy_host, proxy_port, proxy_username,
                proxy_password, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                profile.id,
                profile.protocol.as_str(),
                profile.name,
                profile.host,
                profile.port,
                profile.username,
                profile.password,
                profile.private_key_path,
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
        let drive = self.lock_drive();
        let created_at: String = drive
            .connection
            .query_row(
                "SELECT created_at FROM sessions WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .context("find top_session to update")?;
        let profile = SessionProfile {
            id: id.to_owned(),
            protocol: draft.protocol,
            name: draft.name.trim().to_owned(),
            host: draft.host.trim().to_owned(),
            port: draft.port,
            username: draft.username.trim().to_owned(),
            password: draft.password,
            private_key_path: normalize_private_key_path(draft.private_key_path),
            proxy: draft.proxy,
            created_at,
        };
        let proxy = profile.proxy.as_ref();
        drive.connection.execute(
            "UPDATE sessions SET
                protocol = ?2, name = ?3, host = ?4, port = ?5,
                username = ?6, password = ?7, private_key_path = ?8,
                proxy_host = ?9, proxy_port = ?10, proxy_username = ?11,
                proxy_password = ?12
             WHERE id = ?1",
            params![
                profile.id,
                profile.protocol.as_str(),
                profile.name,
                profile.host,
                profile.port,
                profile.username,
                profile.password,
                profile.private_key_path,
                proxy.map(|value| value.host.as_str()),
                proxy.map(|value| value.port),
                proxy.map(|value| value.username.as_str()),
                proxy.map(|value| value.password.as_str()),
            ],
        )?;
        Ok(profile)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let drive = self.lock_drive();
        drive
            .connection
            .execute("DELETE FROM sessions WHERE id = ?1", [id])
            .context("delete top_session from SQLite")?;
        Ok(())
    }

    pub fn sftp_state(&self, id: &str) -> Result<Option<SftpSessionState>> {
        let drive = self.lock_drive();
        drive
            .connection
            .query_row(
                "SELECT sftp_local_path, sftp_remote_path FROM sessions WHERE id = ?1",
                [id],
                |row| {
                    let local_path = row
                        .get::<_, Option<String>>(0)?
                        .filter(|path| !path.is_empty())
                        .map(PathBuf::from);
                    let remote_path = row
                        .get::<_, Option<String>>(1)?
                        .filter(|path| !path.is_empty());
                    Ok(SftpSessionState {
                        local_path,
                        remote_path,
                    })
                },
            )
            .optional()
            .context("read SFTP session state from SQLite")
    }

    pub fn update_sftp_local_path(&self, id: &str, path: &Path) -> Result<()> {
        let drive = self.lock_drive();
        drive
            .connection
            .execute(
                "UPDATE sessions SET sftp_local_path = ?2 WHERE id = ?1",
                params![id, path.display().to_string()],
            )
            .context("save SFTP local directory to SQLite")?;
        Ok(())
    }

    pub fn update_sftp_remote_path(&self, id: &str, path: &str) -> Result<()> {
        let drive = self.lock_drive();
        drive
            .connection
            .execute(
                "UPDATE sessions SET sftp_remote_path = ?2 WHERE id = ?1",
                params![id, path],
            )
            .context("save SFTP remote directory to SQLite")?;
        Ok(())
    }
}

fn map_session(row: &Row<'_>) -> rusqlite::Result<SessionProfile> {
    let protocol = row
        .get::<_, String>(1)?
        .parse::<Protocol>()
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                Type::Text,
                std::io::Error::new(std::io::ErrorKind::InvalidData, error).into(),
            )
        })?;
    let private_key_path: Option<String> = row.get(7)?;
    let proxy_host: Option<String> = row.get(8)?;
    let proxy = if let Some(host) = proxy_host {
        Some(ProxyConfig {
            host,
            port: row.get::<_, Option<u16>>(9)?.unwrap_or(1080),
            username: row.get::<_, Option<String>>(10)?.unwrap_or_default(),
            password: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
        })
    } else {
        None
    };
    Ok(SessionProfile {
        id: row.get(0)?,
        protocol,
        name: row.get(2)?,
        host: row.get(3)?,
        port: row.get(4)?,
        username: row.get(5)?,
        password: row.get(6)?,
        private_key_path,
        proxy,
        created_at: row.get(12)?,
    })
}

fn normalize_private_key_path(path: Option<String>) -> Option<String> {
    path.and_then(|path| {
        let path = path.trim().to_owned();
        (!path.is_empty()).then_some(path)
    })
}
