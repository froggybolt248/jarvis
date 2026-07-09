use anyhow::Result;
use rusqlite::Connection;

const MIGRATION_001: &str = r#"
CREATE TABLE chunks (
    id TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    heading TEXT,
    content TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_chunks_source_path ON chunks(source_path);

CREATE VIRTUAL TABLE chunks_fts USING fts5(
    heading, content,
    content='chunks',
    content_rowid='rowid'
);

CREATE TRIGGER chunks_ai AFTER INSERT ON chunks BEGIN
    INSERT INTO chunks_fts(rowid, heading, content) VALUES (new.rowid, new.heading, new.content);
END;
CREATE TRIGGER chunks_ad AFTER DELETE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, heading, content) VALUES ('delete', old.rowid, old.heading, old.content);
END;
CREATE TRIGGER chunks_au AFTER UPDATE ON chunks BEGIN
    INSERT INTO chunks_fts(chunks_fts, rowid, heading, content) VALUES ('delete', old.rowid, old.heading, old.content);
    INSERT INTO chunks_fts(rowid, heading, content) VALUES (new.rowid, new.heading, new.content);
END;

CREATE VIRTUAL TABLE chunks_vec USING vec0(
    chunk_rowid INTEGER PRIMARY KEY,
    embedding FLOAT[768]
);

CREATE TABLE core_memory (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    content TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE calendar_events (
    id TEXT PRIMARY KEY,
    google_id TEXT UNIQUE,
    calendar_id TEXT,
    summary TEXT,
    description TEXT,
    location TEXT,
    start_at TEXT,
    end_at TEXT,
    all_day INTEGER DEFAULT 0,
    status TEXT,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_calendar_events_start_at ON calendar_events(start_at);

CREATE TABLE calendar_sync_state (
    calendar_id TEXT PRIMARY KEY,
    sync_token TEXT,
    last_synced_at TEXT
);

CREATE TABLE diet_targets (
    id TEXT PRIMARY KEY,
    effective_date TEXT NOT NULL,
    calories INTEGER,
    protein_g INTEGER,
    carbs_g INTEGER,
    fat_g INTEGER,
    created_at TEXT NOT NULL
);

CREATE TABLE diet_logs (
    id TEXT PRIMARY KEY,
    logged_at TEXT NOT NULL,
    description TEXT NOT NULL,
    calories INTEGER,
    protein_g INTEGER,
    carbs_g INTEGER,
    fat_g INTEGER,
    confidence REAL
);
CREATE INDEX idx_diet_logs_logged_at ON diet_logs(logged_at);

CREATE TABLE gym_programs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    spec_json TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE gym_sessions (
    id TEXT PRIMARY KEY,
    program_id TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    notes TEXT
);

CREATE TABLE gym_sets (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    exercise TEXT NOT NULL,
    weight REAL,
    reps INTEGER,
    rpe REAL,
    set_index INTEGER,
    FOREIGN KEY(session_id) REFERENCES gym_sessions(id) ON DELETE CASCADE
);

CREATE TABLE study_courses (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    code TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE srs_cards (
    id TEXT PRIMARY KEY,
    course_id TEXT,
    front TEXT NOT NULL,
    back TEXT NOT NULL,
    ease_factor REAL NOT NULL DEFAULT 2.5,
    interval_days INTEGER NOT NULL DEFAULT 0,
    repetitions INTEGER NOT NULL DEFAULT 0,
    due_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_srs_cards_due_at ON srs_cards(due_at);

CREATE TABLE quiet_feed (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    deep_link TEXT,
    source TEXT
);
CREATE INDEX idx_quiet_feed_created_at ON quiet_feed(created_at);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE scheduler_jobs (
    id TEXT PRIMARY KEY,
    job_kind TEXT NOT NULL,
    cron TEXT,
    last_run_at TEXT,
    next_run_at TEXT,
    enabled INTEGER NOT NULL DEFAULT 1
);
"#;

/// Run all pending migrations, tracked via `PRAGMA user_version`. Idempotent:
/// calling this on an already up-to-date database is a no-op.
pub fn run(conn: &Connection) -> Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;

    if version < 1 {
        conn.execute_batch(MIGRATION_001)?;
        conn.pragma_update(None, "user_version", 1i64)?;
    }

    Ok(())
}
