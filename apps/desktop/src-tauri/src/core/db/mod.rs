// Owned by WP-DB: SQLite connection, migrations, typed queries.

mod migrations;
pub mod queries;

use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub use queries::search::SearchHit;

/// Wraps a single SQLite connection (this is a desktop app; one writer is
/// sufficient) behind a mutex so `Db` can be shared across threads/commands.
pub struct Db {
    conn: Mutex<Connection>,
}

// Ensure the sqlite-vec extension is registered with SQLite exactly once,
// before any connection is opened. `sqlite3_auto_extension` applies to every
// connection opened by the process afterwards.
fn register_sqlite_vec() {
    use std::sync::Once;
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *mut std::os::raw::c_char,
                *const rusqlite::ffi::sqlite3_api_routines,
            ) -> std::os::raw::c_int,
        >(sqlite_vec::sqlite3_vec_init as *const ())));
    });
}

impl Db {
    /// Open (creating if necessary) a database file at `path`, applying
    /// pragmas and running any pending migrations.
    pub fn open(path: &Path) -> Result<Db> {
        register_sqlite_vec();
        let conn = Connection::open(path)
            .with_context(|| format!("opening sqlite db at {}", path.display()))?;
        Self::init_connection(conn)
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> Result<Db> {
        register_sqlite_vec();
        let conn = Connection::open_in_memory().context("opening in-memory sqlite db")?;
        Self::init_connection(conn)
    }

    fn init_connection(conn: Connection) -> Result<Db> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("setting journal_mode")?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .context("setting foreign_keys")?;
        conn.pragma_update(None, "busy_timeout", 5000i64)
            .context("setting busy_timeout")?;

        migrations::run(&conn).context("running migrations")?;

        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    /// Run a closure with exclusive access to the underlying connection.
    pub(crate) fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        f(&conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("jarvis.db");

        {
            let db = Db::open(&path).expect("first open should apply migrations");
            db.with_conn(|conn| {
                let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
                assert_eq!(version, 1);
                Ok(())
            })
            .unwrap();
        }

        {
            let db = Db::open(&path).expect("second open should be idempotent");
            db.with_conn(|conn| {
                let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
                assert_eq!(version, 1);
                let count: i64 = conn.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='chunks'",
                    [],
                    |r| r.get(0),
                )?;
                assert_eq!(count, 1);
                Ok(())
            })
            .unwrap();
        }
    }

    #[test]
    fn fts5_is_available() {
        let db = Db::open_in_memory().unwrap();
        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO chunks (id, source_path, heading, content, updated_at) \
                 VALUES ('c1', 'notes.md', 'Title', 'hello world from fts5', '2026-01-01T00:00:00Z')",
                [],
            )?;
            let content: String = conn.query_row(
                "SELECT content FROM chunks_fts WHERE chunks_fts MATCH 'fts5' LIMIT 1",
                [],
                |r| r.get(0),
            )?;
            assert!(content.contains("fts5"));
            Ok(())
        })
        .expect("FTS5 must be available in the bundled sqlite build");
    }

    #[test]
    fn vec0_smoke_test() {
        let db = Db::open_in_memory().unwrap();
        db.with_conn(|conn| {
            let embedding: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
            let bytes = queries::search::serialize_f32(&embedding);
            conn.execute(
                "INSERT INTO chunks_vec (chunk_rowid, embedding) VALUES (1, ?1)",
                rusqlite::params![bytes],
            )?;

            let query_bytes = queries::search::serialize_f32(&embedding);
            let rowid: i64 = conn.query_row(
                "SELECT chunk_rowid FROM chunks_vec WHERE embedding MATCH ?1 AND k = 1 ORDER BY distance",
                rusqlite::params![query_bytes],
                |r| r.get(0),
            )?;
            assert_eq!(rowid, 1);
            Ok(())
        })
        .expect("sqlite-vec vec0 virtual table must work");
    }
}
