use std::fs;

use anyhow::{Context as _, Result};
use rusqlite::{Connection, TransactionBehavior};

pub struct SqliteDrive {
    pub(crate) connection: Connection,
}

impl SqliteDrive {
    pub fn new() -> Result<Self> {
        fs::create_dir_all(&crate::infrastructure::paths::get().data)
            .context("create database directory")?;

        let connection = Connection::open(
            crate::infrastructure::paths::get()
                .data
                .join("gpui_remote.db"),
        )
        .context("open SQLite database")?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .context("configure SQLite connection")?;

        let mut drive = Self { connection };
        drive.init_table()?;
        Ok(drive)
    }

    fn init_table(&mut self) -> Result<()> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS session_groups (
                    id TEXT PRIMARY KEY NOT NULL,
                    name TEXT NOT NULL,
                    parent_id TEXT REFERENCES session_groups(id)
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS session_groups_sibling_name_idx
                    ON session_groups(COALESCE(parent_id, ''), name COLLATE NOCASE);
                 CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY NOT NULL,
                    protocol TEXT NOT NULL DEFAULT 'SSH_AND_SFTP',
                    name TEXT NOT NULL,
                    host TEXT NOT NULL,
                    port INTEGER NOT NULL,
                    username TEXT NOT NULL,
                    password TEXT NOT NULL DEFAULT '',
                    private_key_path TEXT,
                    proxy_host TEXT,
                    proxy_port INTEGER,
                    proxy_username TEXT,
                    proxy_password TEXT,
                    ssh_tunnel_remote_port INTEGER,
                    ssh_tunnel_forward_address TEXT,
                    ssh_tunnel_forward_port INTEGER,
                    sftp_local_path TEXT,
                    sftp_remote_path TEXT,
                    created_at TEXT NOT NULL,
                    group_id TEXT REFERENCES session_groups(id) ON DELETE SET NULL
                 );
                 CREATE INDEX IF NOT EXISTS sessions_created_at_idx
                    ON sessions(created_at DESC);",
            )
            .context("initialize SQLite tables")?;

        let has_group_id: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('sessions') WHERE name = 'group_id')",
            [],
            |row| row.get(0),
        )?;
        if !has_group_id {
            transaction.execute(
                "ALTER TABLE sessions ADD COLUMN group_id TEXT REFERENCES session_groups(id) ON DELETE SET NULL",
                [],
            ).context("add session group column")?;
        }
        transaction.commit()?;

        let _ = self.connection.execute(
            "ALTER TABLE sessions ADD COLUMN protocol TEXT NOT NULL DEFAULT 'SSH_AND_SFTP'",
            [],
        );
        let _ = self
            .connection
            .execute("ALTER TABLE sessions ADD COLUMN private_key_path TEXT", []);
        let _ = self.connection.execute(
            "ALTER TABLE sessions ADD COLUMN ssh_tunnel_remote_port INTEGER",
            [],
        );
        let _ = self.connection.execute(
            "ALTER TABLE sessions ADD COLUMN ssh_tunnel_forward_address TEXT",
            [],
        );
        let _ = self.connection.execute(
            "ALTER TABLE sessions ADD COLUMN ssh_tunnel_forward_port INTEGER",
            [],
        );
        let _ = self
            .connection
            .execute("ALTER TABLE sessions ADD COLUMN sftp_local_path TEXT", []);
        let _ = self
            .connection
            .execute("ALTER TABLE sessions ADD COLUMN sftp_remote_path TEXT", []);
        Ok(())
    }
}
