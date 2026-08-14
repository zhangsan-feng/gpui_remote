use std::fs;

use anyhow::{Context as _, Result};
use rusqlite::Connection;

const DEFAULT_DATABASE_PATH: &str = "data/gpui_remote.db";

pub struct SqliteDrive {
    pub(crate) connection: Connection,
}

impl SqliteDrive {
    pub fn new() -> Result<Self> {
        fs::create_dir_all("data").context("create database directory")?;

        let connection = Connection::open(DEFAULT_DATABASE_PATH).context("open SQLite database")?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .context("configure SQLite connection")?;

        let drive = Self { connection };
        drive.init_table()?;
        Ok(drive)
    }

    pub fn init_table(&self) -> Result<()> {
        self.connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY NOT NULL,
                    protocol TEXT NOT NULL DEFAULT 'SSH',
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
                    created_at TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS sessions_created_at_idx
                    ON sessions(created_at DESC);",
            )
            .context("initialize SQLite tables")?;

        let _ = self.connection.execute(
            "ALTER TABLE sessions ADD COLUMN protocol TEXT NOT NULL DEFAULT 'SSH'",
            [],
        );
        let _ = self
            .connection
            .execute("ALTER TABLE sessions ADD COLUMN private_key_path TEXT", []);
        Ok(())
    }
}
