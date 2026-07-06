use crate::api::Store;
use crate::error::{Error, Result};
use crate::store::sqlite::migrations;
use rusqlite::Connection;
use std::path::Path;

/// SQLite-backed [`Store`] implementation.
///
/// Owns a single [`Connection`]. The connection is opened in WAL mode with
/// the same PRAGMAs Kupo uses (`page_size=32768`, `cache_size=1024`,
/// `synchronous=NORMAL`, `journal_mode=WAL`), migrations are applied, and
/// `foreign_keys=ON` is enabled before the store is returned.
#[derive(Debug)]
pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    /// Open (or create) a database file at `path` and run any pending
    /// migrations. The parent directory must already exist.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if path.as_os_str().is_empty() {
            return Err(Error::InvalidLocation("database path is empty".to_string()));
        }

        let conn = Connection::open(path)?;
        let store = Self::initialize(conn)?;
        Ok(store)
    }

    /// Open a private in-memory database. Convenient for tests and
    /// short-lived processes; the database lives only as long as the
    /// returned [`SqliteStore`].
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::initialize(conn)
    }

    /// Apply PRAGMAs and run pending migrations on a freshly opened
    /// connection. This is the equivalent of Kupo's
    /// `withLongLivedConnection` setup block.
    fn initialize(mut conn: Connection) -> Result<Self> {
        conn.execute_batch(
            r"
            PRAGMA page_size = 32768;
            PRAGMA cache_size = 1024;
            PRAGMA synchronous = NORMAL;
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            ",
        )?;

        let current = migrations::database_version(&conn)?;
        migrations::run_migrations(&mut conn, current)?;

        Ok(Self { conn })
    }
}

impl Store for SqliteStore {
    fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use migrations::database_version;

    #[test]
    fn open_in_memory_creates_fresh_schema() {
        let store = SqliteStore::open_in_memory().unwrap();
        assert_eq!(
            database_version(store.connection()).unwrap(),
            migrations::latest_revision()
        );
    }

    #[test]
    fn open_persists_migrations_to_disk() {
        let dir = tempdir();
        let path = dir.join("indexer.sqlite3");

        {
            let store = SqliteStore::open(&path).unwrap();
            assert_eq!(
                database_version(store.connection()).unwrap(),
                migrations::latest_revision()
            );
        }

        // Reopening must observe the same version and not re-run migrations.
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(
            database_version(store.connection()).unwrap(),
            migrations::latest_revision()
        );
    }

    #[test]
    fn open_rejects_empty_path() {
        let err = SqliteStore::open("").unwrap_err();
        assert!(matches!(err, Error::InvalidLocation(_)));
    }

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir();
        let unique = format!(
            "konduit-indexer-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = base.join(unique);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
