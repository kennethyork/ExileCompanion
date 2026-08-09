use anyhow::Result;
use poe_core::GameEvent;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct EventStore(Connection);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSnapshotRecord {
    pub id: i64,
    pub captured_at: String,
    pub label: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterProfileRecord {
    pub profile_id: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapRunRecord {
    pub captured_at: String,
    pub area: String,
    pub duration_seconds: u64,
    pub deaths: u32,
    pub investment: String,
    pub loot: String,
}

impl EventStore {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY,
                occurred_at TEXT NOT NULL,
                kind TEXT NOT NULL,
                message TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS character_snapshots (
                id INTEGER PRIMARY KEY,
                captured_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                label TEXT NOT NULL,
                data TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS character_profiles (
                profile_id TEXT PRIMARY KEY,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                data TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS app_preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS map_runs (
                id INTEGER PRIMARY KEY,
                captured_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                area TEXT NOT NULL,
                duration_seconds INTEGER NOT NULL,
                deaths INTEGER NOT NULL,
                investment TEXT NOT NULL,
                loot TEXT NOT NULL
             );",
        )?;
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

    pub fn record_character_snapshot(&self, label: &str, data: &str) -> Result<i64> {
        self.0.execute(
            "INSERT INTO character_snapshots (label, data) VALUES (?1, ?2)",
            params![label, data],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn character_snapshots(&self, limit: usize) -> Result<Vec<CharacterSnapshotRecord>> {
        let mut statement = self.0.prepare(
            "SELECT id, captured_at, label, data
             FROM character_snapshots
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit.min(100) as i64], |row| {
            Ok(CharacterSnapshotRecord {
                id: row.get(0)?,
                captured_at: row.get(1)?,
                label: row.get(2)?,
                data: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_character_profile(&self, profile_id: &str, data: &str) -> Result<()> {
        self.0.execute(
            "INSERT INTO character_profiles (profile_id, data)
             VALUES (?1, ?2)
             ON CONFLICT(profile_id) DO UPDATE SET
                data = excluded.data,
                updated_at = CURRENT_TIMESTAMP",
            params![profile_id, data],
        )?;
        Ok(())
    }

    pub fn character_profiles(&self) -> Result<Vec<CharacterProfileRecord>> {
        let mut statement = self.0.prepare(
            "SELECT profile_id, data
             FROM character_profiles
             ORDER BY updated_at DESC, profile_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(CharacterProfileRecord {
                profile_id: row.get(0)?,
                data: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn set_preference(&self, key: &str, value: &str) -> Result<()> {
        self.0.execute(
            "INSERT INTO app_preferences (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn preference(&self, key: &str) -> Result<Option<String>> {
        let mut statement = self
            .0
            .prepare("SELECT value FROM app_preferences WHERE key = ?1")?;
        let mut rows = statement.query([key])?;
        Ok(rows.next()?.map(|row| row.get(0)).transpose()?)
    }

    pub fn record_map_run(&self, run: &MapRunRecord) -> Result<()> {
        self.0.execute(
            "INSERT INTO map_runs (area, duration_seconds, deaths, investment, loot)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &run.area,
                run.duration_seconds,
                run.deaths,
                &run.investment,
                &run.loot
            ],
        )?;
        Ok(())
    }

    pub fn map_runs(&self, limit: usize) -> Result<Vec<MapRunRecord>> {
        let mut statement = self.0.prepare(
            "SELECT captured_at, area, duration_seconds, deaths, investment, loot
             FROM map_runs ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit.min(100) as i64], |row| {
            Ok(MapRunRecord {
                captured_at: row.get(0)?,
                area: row.get(1)?,
                duration_seconds: row.get(2)?,
                deaths: row.get(3)?,
                investment: row.get(4)?,
                loot: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_loads_character_snapshots() {
        let store = EventStore::open(Path::new(":memory:")).unwrap();
        let id = store
            .record_character_snapshot("Level 90 Witch", r#"{"level":90}"#)
            .unwrap();
        let snapshots = store.character_snapshots(10).unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, id);
        assert_eq!(snapshots[0].label, "Level 90 Witch");
        assert_eq!(snapshots[0].data, r#"{"level":90}"#);
    }

    #[test]
    fn upserts_character_profiles() {
        let store = EventStore::open(Path::new(":memory:")).unwrap();
        store
            .save_character_profile("local-1", r#"{"name":"First"}"#)
            .unwrap();
        store
            .save_character_profile("local-1", r#"{"name":"Updated"}"#)
            .unwrap();
        store
            .save_character_profile("local-2", r#"{"name":"Second"}"#)
            .unwrap();
        let profiles = store.character_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
        assert!(profiles.iter().any(|profile| {
            profile.profile_id == "local-1" && profile.data.contains("Updated")
        }));
    }

    #[test]
    fn stores_preferences() {
        let store = EventStore::open(Path::new(":memory:")).unwrap();
        assert_eq!(store.preference("hud.opacity").unwrap(), None);
        store.set_preference("hud.opacity", "0.85").unwrap();
        assert_eq!(
            store.preference("hud.opacity").unwrap().as_deref(),
            Some("0.85")
        );
    }

    #[test]
    fn stores_map_runs() {
        let store = EventStore::open(Path::new(":memory:")).unwrap();
        store
            .record_map_run(&MapRunRecord {
                captured_at: String::new(),
                area: "Dunes".into(),
                duration_seconds: 123,
                deaths: 1,
                investment: "4 chaos".into(),
                loot: "12 chaos".into(),
            })
            .unwrap();
        let runs = store.map_runs(10).unwrap();
        assert_eq!(runs[0].area, "Dunes");
        assert_eq!(runs[0].duration_seconds, 123);
    }
}
