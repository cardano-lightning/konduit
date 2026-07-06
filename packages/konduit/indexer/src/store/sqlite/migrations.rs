use crate::error::{Error, Result};
use rusqlite::{Connection, Transaction};

/// A revision number recorded in `PRAGMA user_version` after a successful migration.
pub type Revision = u32;
/// A single schema revision: a raw SQL blob applied verbatim inside one
/// transaction, followed by a `PRAGMA user_version = N` that records the
/// new schema version.
struct Migration {
    version: Revision,
    sql: &'static str,
}

/// All known migrations, ordered by ascending [`Revision`].
static MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("../../../db/v1.0.0/001.sql"),
}];

/// The highest [`Revision`] known to this binary. Migrations are expected
/// to bring a database at any prior version up to this one.
pub fn latest_revision() -> Revision {
    MIGRATIONS.last().map(|m| m.version).unwrap_or(0)
}

/// Read the schema version stored in `PRAGMA user_version`.
///
/// Fresh (empty) SQLite databases report `0` by default, which means
/// *every* migration in [`MIGRATIONS`] is pending.
pub fn database_version(conn: &Connection) -> Result<Revision> {
    let version = conn.query_row("PRAGMA user_version", [], |row| row.get::<_, Revision>(0))?;
    Ok(version)
}

/// Run every migration whose revision number is strictly greater than
/// `current_version`. Each migration runs inside its own transaction.
///
/// We let rusqlite's `execute_batch` handle the statement splitting for the
/// migration body (so trigger `BEGIN...END;` blocks round-trip correctly).
/// The `PRAGMA user_version = N;` line is run last so the new schema version
/// is only recorded once the migration's own statements have succeeded.
pub fn run_migrations(conn: &mut Connection, current_version: Revision) -> Result<()> {
    let pending: Vec<&Migration> = MIGRATIONS
        .iter()
        .filter(|m| m.version > current_version)
        .collect();

    if pending.is_empty() {
        return Ok(());
    }

    for migration in pending {
        execute_migration(conn, migration)?;
    }

    Ok(())
}

fn execute_migration(conn: &mut Connection, migration: &Migration) -> Result<()> {
    let tx: Transaction<'_> = conn.transaction()?;

    tx.execute_batch(migration.sql).map_err(|e| {
        Error::Migration(format!(
            "failed to apply migration to version {}: {e}",
            migration.version
        ))
    })?;

    let record_version = format!("PRAGMA user_version = {};", migration.version);
    tx.execute(&record_version, []).map_err(|e| {
        Error::Migration(format!(
            "failed to record PRAGMA user_version = {}: {e}",
            migration.version
        ))
    })?;

    tx.commit().map_err(|e| {
        Error::Migration(format!(
            "failed to commit migration to version {}: {e}",
            migration.version
        ))
    })?;

    Ok(())
}

#[cfg(test)]
mod migrations_test_suite {
    use super::*;

    #[test]
    fn fresh_database_reports_version_zero() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(database_version(&conn).unwrap(), 0);
    }

    #[test]
    fn no_pending_migration_is_a_no_op() {
        // Pretend the DB is already up-to-date by recording the latest version.
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!("PRAGMA user_version = {};", latest_revision()))
            .unwrap();

        let before = database_version(&conn).unwrap();
        run_migrations(&mut conn, before).unwrap();
        let after = database_version(&conn).unwrap();

        // No migration should have run, so the version is unchanged.
        assert_eq!(before, after);
        assert_eq!(after, latest_revision());
    }

    #[test]
    fn migration_is_idempotent_on_rerun() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn, 0).unwrap();
        let after_first = database_version(&conn).unwrap();

        // Re-running with the current version recorded must be a no-op
        // (and must not error out, e.g. by trying to re-create tables).
        run_migrations(&mut conn, after_first).unwrap();
        assert_eq!(database_version(&conn).unwrap(), after_first);
    }

    #[test]
    fn migration_creates_expected_tables_and_trigger() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn, 0).unwrap();

        let names: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','trigger') ORDER BY type, name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();

        assert!(names.iter().any(|n| n == "block"));
        assert!(names.iter().any(|n| n == "channel"));
        assert!(names.iter().any(|n| n == "step"));
        assert!(names.iter().any(|n| n == "no_step_after_close"));
    }
}
