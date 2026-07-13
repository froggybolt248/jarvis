// WP-Agent-Tools owns this file.

//! `log_workout` (mutating) and `get_gym_recent` (read-only): the
//! agent-facing gym tools. The mutating tool creates one session and its
//! sets (via the shared `queries::gym` helper also used by the quick-add
//! form command) and logs a Quiet Feed audit row.

use anyhow::{Context, Result};
use serde_json::Value;

use crate::core::agent::provider::ToolDef;
use crate::core::db::queries::gym::{self, SetInput};
use crate::core::db::queries::quiet_feed::QuietFeedItem;
use crate::core::db::Db;

use super::{Tool, ToolContext};

const DEFAULT_LIMIT: usize = 5;
const MAX_LIMIT: usize = 20;

/// Extract an optional non-negative integer field, clamped to `[1, max]`,
/// defaulting to `default` when absent.
fn optional_k(args: &Value, field: &str, default: usize, max: usize) -> Result<usize> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let n = v
                .as_u64()
                .with_context(|| format!("field '{field}' must be a non-negative integer"))?;
            Ok((n as usize).clamp(1, max))
        }
    }
}

/// Exercise names (in `set_index` order, one entry per set) logged under a
/// given gym session. There's no shared `queries::gym` helper for this yet,
/// so this reads directly via `Db::with_conn` (a `pub(crate)` escape hatch)
/// rather than duplicating session/set plumbing.
fn exercises_for_session(db: &Db, session_id: &str) -> Result<Vec<String>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT exercise FROM gym_sets WHERE session_id = ?1 ORDER BY set_index ASC",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![session_id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

/// Extract a required string field from a JSON object.
fn required_str<'a>(args: &'a Value, field: &str) -> Result<&'a str> {
    args.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing or non-string required field '{field}'"))
}

/// Extract an optional string field from a JSON object.
fn optional_str(args: &Value, field: &str) -> Option<String> {
    args.get(field).and_then(Value::as_str).map(str::to_string)
}

/// Extract an optional f64 field (accepts both JSON numbers and integers).
fn optional_f64(args: &Value, field: &str) -> Result<Option<f64>> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("field '{field}' must be a number")),
    }
}

/// Extract an optional integer field.
fn optional_i64(args: &Value, field: &str) -> Result<Option<i64>> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("field '{field}' must be an integer")),
    }
}

/// Logs a completed workout: one session plus one or more sets.
pub struct LogWorkout;

#[async_trait::async_trait]
impl Tool for LogWorkout {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "log_workout".to_string(),
            description: "Log a completed workout: creates one gym session with the given sets. \
                This mutates the user's gym log, so only use it when the user has clearly \
                described a workout they did."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "notes": {
                        "type": "string",
                        "description": "Optional free-text notes about the session."
                    },
                    "sets": {
                        "type": "array",
                        "description": "The sets performed, in order.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "exercise": {
                                    "type": "string",
                                    "description": "Exercise name, e.g. 'Squat'."
                                },
                                "weight": {
                                    "type": "number",
                                    "description": "Weight used, in the user's usual unit."
                                },
                                "reps": {
                                    "type": "integer",
                                    "description": "Repetitions performed."
                                },
                                "rpe": {
                                    "type": "number",
                                    "description": "Rate of perceived exertion (e.g. 0-10)."
                                }
                            },
                            "required": ["exercise"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["sets"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let notes = optional_str(args, "notes");

        let sets_val = args
            .get("sets")
            .and_then(Value::as_array)
            .context("missing or non-array required field 'sets'")?;
        if sets_val.is_empty() {
            anyhow::bail!("field 'sets' must be non-empty");
        }

        let mut sets = Vec::with_capacity(sets_val.len());
        let mut exercises: Vec<String> = Vec::new();
        for set_val in sets_val {
            let exercise = required_str(set_val, "exercise")?.to_string();
            let weight = optional_f64(set_val, "weight")?;
            let reps = optional_i64(set_val, "reps")?;
            let rpe = optional_f64(set_val, "rpe")?;
            if !exercises.contains(&exercise) {
                exercises.push(exercise.clone());
            }
            sets.push(SetInput { exercise, weight, reps, rpe });
        }
        let set_count = sets.len();
        let exercise_count = exercises.len();

        let session = gym::log_workout(ctx.db, notes, sets)?;

        let item = QuietFeedItem {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            kind: "gym".to_string(),
            title: format!("Logged {set_count} sets across {exercise_count} exercises"),
            body: Some(exercises.join(", ")),
            deep_link: None,
            source: Some("log_workout".to_string()),
        };
        ctx.db.insert_feed(&item)?;

        Ok(format!(
            "Logged workout session {} with {set_count} sets across {exercise_count} exercises.",
            session.id
        ))
    }
}

/// Read-only: the most recent gym sessions, with duration (if computable),
/// notes, and a set-count/exercises summary.
pub struct GetGymRecent;

#[async_trait::async_trait]
impl Tool for GetGymRecent {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "get_gym_recent".to_string(),
            description: "Get the user's most recent gym sessions, with dates, durations, notes, \
                and exercises performed. Use this before answering any question about the user's \
                recent training or workout history."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Max number of sessions to return (default 5, max 20).",
                        "minimum": 1,
                        "maximum": MAX_LIMIT
                    }
                },
                "required": [],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let limit = optional_k(args, "limit", DEFAULT_LIMIT, MAX_LIMIT)?;

        let sessions = ctx.db.recent_sessions(limit)?;
        if sessions.is_empty() {
            return Ok("No gym sessions logged yet.".to_string());
        }

        let mut lines = Vec::with_capacity(sessions.len());
        for session in &sessions {
            let date = session
                .started_at
                .split('T')
                .next()
                .unwrap_or(&session.started_at);

            let duration = match (
                chrono::DateTime::parse_from_rfc3339(&session.started_at),
                session
                    .ended_at
                    .as_deref()
                    .and_then(|e| chrono::DateTime::parse_from_rfc3339(e).ok()),
            ) {
                (Ok(start), Some(end)) => {
                    let mins = (end - start).num_minutes();
                    if mins >= 0 {
                        format!(" ({mins}m)")
                    } else {
                        String::new()
                    }
                }
                _ => String::new(),
            };

            let exercises = exercises_for_session(ctx.db, &session.id)?;
            let set_count = exercises.len();
            let mut unique = Vec::new();
            for e in &exercises {
                if !unique.contains(e) {
                    unique.push(e.clone());
                }
            }
            let exercises_summary = if unique.is_empty() {
                "no sets logged".to_string()
            } else {
                format!("{set_count} sets across {}", unique.join(", "))
            };

            let notes = session
                .notes
                .as_deref()
                .map(|n| format!(" \u{2014} {n}"))
                .unwrap_or_default();

            lines.push(format!("- {date}{duration}: {exercises_summary}{notes}"));
        }

        Ok(format!("{} session(s):\n{}", sessions.len(), lines.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::provider::{ChatEvent, ChatMessage, ChatOptions, ChatProvider, ProviderHealth};
    use crate::core::db::Db;
    use crate::core::memory::Vault;
    use futures_util::stream::BoxStream;
    use tempfile::tempdir;

    struct StubProvider;

    #[async_trait::async_trait]
    impl ChatProvider for StubProvider {
        async fn chat_stream(
            &self,
            _messages: Vec<ChatMessage>,
            _opts: ChatOptions,
        ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatEvent>>> {
            anyhow::bail!("not implemented in stub")
        }

        async fn embed(&self, _texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
            anyhow::bail!("not implemented in stub")
        }

        async fn health(&self) -> anyhow::Result<ProviderHealth> {
            anyhow::bail!("not implemented in stub")
        }
    }

    #[tokio::test]
    async fn log_workout_inserts_session_sets_and_feed_row() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = LogWorkout;
        let args = serde_json::json!({
            "notes": "leg day",
            "sets": [
                {"exercise": "Squat", "weight": 100.0, "reps": 5, "rpe": 8},
                {"exercise": "Squat", "weight": 105.0, "reps": 3}
            ]
        });
        let result = tool.execute(&ctx, &args).await.unwrap();
        assert!(result.contains("2 sets"));
        assert!(result.contains("1 exercises"));

        let sessions = db.recent_sessions(10).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].notes.as_deref(), Some("leg day"));

        let sets = db.sets_for_exercise("Squat", 10).unwrap();
        assert_eq!(sets.len(), 2);

        let feed = db.recent_feed(10).unwrap();
        assert_eq!(feed.len(), 1);
        assert_eq!(feed[0].kind, "gym");
        assert_eq!(feed[0].source.as_deref(), Some("log_workout"));
    }

    #[tokio::test]
    async fn log_workout_missing_sets_errors() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = LogWorkout;
        let args = serde_json::json!({});
        assert!(tool.execute(&ctx, &args).await.is_err());
    }

    #[tokio::test]
    async fn log_workout_empty_sets_errors() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = LogWorkout;
        let args = serde_json::json!({"sets": []});
        assert!(tool.execute(&ctx, &args).await.is_err());
    }

    #[tokio::test]
    async fn get_gym_recent_reports_sessions_with_exercises() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let log = LogWorkout;
        let log_args = serde_json::json!({
            "notes": "leg day",
            "sets": [
                {"exercise": "Squat", "weight": 100.0, "reps": 5},
                {"exercise": "Deadlift", "weight": 140.0, "reps": 3}
            ]
        });
        log.execute(&ctx, &log_args).await.unwrap();

        let tool = GetGymRecent;
        let result = tool.execute(&ctx, &serde_json::json!({})).await.unwrap();

        assert!(result.contains("1 session(s)"), "got: {result}");
        assert!(result.contains("2 sets across Squat, Deadlift"), "got: {result}");
        assert!(result.contains("leg day"), "got: {result}");
    }

    #[tokio::test]
    async fn get_gym_recent_reports_none_when_empty() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = GetGymRecent;
        let result = tool.execute(&ctx, &serde_json::json!({})).await.unwrap();
        assert_eq!(result, "No gym sessions logged yet.");
    }
}
