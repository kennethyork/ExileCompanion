use anyhow::Result;
use poe_core::GameEvent;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct EventStore(Connection);

impl EventStore {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY, occurred_at TEXT NOT NULL, kind TEXT NOT NULL, message TEXT NOT NULL);")?;
        Ok(Self(connection))
    }

    pub fn record(&self, event: &GameEvent) -> Result<()> {
        self.0.execute(
            "INSERT INTO events (occurred_at, kind, message) VALUES (?1, ?2, ?3)",
            params![
                event.occurred_at.to_rfc3339(),
                format!("{:?}", event.kind),
                event.message
            ],
        )?;
        Ok(())
    }
}
