//! Settle an interrupted write on an existing metadata database (Spec 108
//! step 4, hot journal).
//!
//! Core keeps `meta.sqlite3` in SQLite's default rollback-journal (delete)
//! mode. A crash during a transaction leaves a hot `meta.sqlite3-journal`
//! holding the original content of pages the **uncommitted** transaction
//! changed; SQLite's rule, and Core's next open, roll it back. This function
//! performs exactly that rollback and nothing else: no create flag, no
//! directory creation, no migrations.

use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, OpenFlags};

/// Why the interrupted write could not be settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettleJournalError {
    /// Another connection holds RESERVED or a stronger lock (a live writer).
    Busy,
    /// Missing file, permission, I/O or any other SQLite failure.
    Io,
}

fn classify(error: &rusqlite::Error) -> SettleJournalError {
    match error.sqlite_error_code() {
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => SettleJournalError::Busy,
        _ => SettleJournalError::Io,
    }
}

/// Let SQLite roll back a hot journal on `root/meta.sqlite3`.
///
/// Opens the existing file read-write without the create flag, reads once
/// (which plays back a hot journal), then takes and releases a RESERVED lock
/// so a live writer is reported as [`SettleJournalError::Busy`] rather than
/// mistaken for a crash.
///
/// # Errors
/// [`SettleJournalError`] when the file is missing, busy or unreadable.
pub fn settle_meta_journal(root: &Path) -> Result<(), SettleJournalError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn =
        Connection::open_with_flags(root.join("meta.sqlite3"), flags).map_err(|e| classify(&e))?;
    conn.busy_timeout(Duration::ZERO)
        .map_err(|e| classify(&e))?;
    conn.query_row("SELECT COUNT(*) FROM sqlite_master", [], |r| {
        r.get::<_, i64>(0)
    })
    .map_err(|e| classify(&e))?;
    conn.execute_batch("BEGIN IMMEDIATE; ROLLBACK;")
        .map_err(|e| classify(&e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteMetaStore;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "medscale-settle-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    #[test]
    fn core_meta_store_uses_delete_journal_mode() {
        // The desktop recovery allowlist assumes delete-mode journals; if
        // Core ever changes journal mode this must be revisited.
        let dir = scratch("mode");
        drop(SqliteMetaStore::open(&dir).expect("meta"));
        let conn = Connection::open(dir.join("meta.sqlite3")).expect("db");
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .expect("mode");
        assert_eq!(mode.to_ascii_lowercase(), "delete");
        drop(conn);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn settle_never_creates_or_migrates() {
        let dir = scratch("missing");
        assert_eq!(settle_meta_journal(&dir), Err(SettleJournalError::Io));
        assert!(!dir.join("meta.sqlite3").exists(), "no create flag");

        drop(SqliteMetaStore::open(&dir).expect("meta"));
        let conn = Connection::open(dir.join("meta.sqlite3")).expect("db");
        let before: i64 = conn
            .query_row("SELECT COUNT(*) FROM migration_journal", [], |r| r.get(0))
            .expect("count");
        conn.execute("DELETE FROM migration_journal WHERE rowid = (SELECT MAX(rowid) FROM migration_journal)", [])
            .expect("drop one migration row");
        drop(conn);
        settle_meta_journal(&dir).expect("settle");
        let conn = Connection::open(dir.join("meta.sqlite3")).expect("db");
        let after: i64 = conn
            .query_row("SELECT COUNT(*) FROM migration_journal", [], |r| r.get(0))
            .expect("count");
        assert_eq!(after, before - 1, "settle must not run migrations");
        drop(conn);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_live_reserved_writer_is_busy_not_a_crash() {
        let dir = scratch("reserved");
        drop(SqliteMetaStore::open(&dir).expect("meta"));
        let writer = Connection::open(dir.join("meta.sqlite3")).expect("db");
        writer
            .execute_batch("BEGIN IMMEDIATE; INSERT INTO store_state(key, value) VALUES ('settle_probe', '1');")
            .expect("reserved");
        assert_eq!(settle_meta_journal(&dir), Err(SettleJournalError::Busy));
        writer.execute_batch("ROLLBACK;").expect("release");
        drop(writer);
        settle_meta_journal(&dir).expect("free");
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }
}
