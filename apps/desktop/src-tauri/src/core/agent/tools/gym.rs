// WP-Agent-Tools owns this file.

//! `log_workout`: the agent-facing gym tool. Creates one session and its
//! sets (via the shared `queries::gym` helper also used by the quick-add
//! form command) and logs a Quiet Feed audit row.

use anyhow::{Context, Result};
use serde_json::Value;

use crate::core::agent::provider::ToolDef;
use crate::core::db::queries::gym::{self, SetInput};
use crate::core::db::queries::quiet_feed::QuietFeedItem;

use super::{Tool, ToolContext};

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
}
