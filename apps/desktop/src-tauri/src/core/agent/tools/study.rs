// WP-Agent-Tools owns this file.

//! `create_study_card` and `review_study_card` (mutating), plus
//! `get_due_cards` (read-only): the agent-facing spaced-repetition tools.
//! The mutating tools delegate to the shared `queries::study` helpers (also
//! used by the quick-add form commands) and log a Quiet Feed audit row.

use anyhow::Result;
use serde_json::Value;

use crate::core::agent::provider::ToolDef;
use crate::core::db::queries::quiet_feed::QuietFeedItem;
use crate::core::db::queries::study;

use super::{Tool, ToolContext};

const MAX_DUE_CARDS: usize = 20;

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

/// Extract a required integer field, clamped to `[0, 5]`.
fn required_quality(args: &Value, field: &str) -> Result<u8> {
    let n = args
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("missing or non-integer required field '{field}'"))?;
    Ok(n.clamp(0, 5) as u8)
}

/// Creates a new spaced-repetition study card, due immediately.
pub struct CreateStudyCard;

#[async_trait::async_trait]
impl Tool for CreateStudyCard {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "create_study_card".to_string(),
            description: "Create a new spaced-repetition study card (front/back), due \
                immediately. This mutates the user's study deck, so only use it when the user \
                has clearly asked to add a card."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "front": {
                        "type": "string",
                        "description": "The question/prompt side of the card."
                    },
                    "back": {
                        "type": "string",
                        "description": "The answer side of the card."
                    },
                    "course_id": {
                        "type": "string",
                        "description": "Optional course/deck this card belongs to."
                    }
                },
                "required": ["front", "back"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let front = required_str(args, "front")?.to_string();
        let back = required_str(args, "back")?.to_string();
        let course_id = optional_str(args, "course_id");

        let card = study::create_study_card(ctx.db, front.clone(), back.clone(), course_id)?;

        let item = QuietFeedItem {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            kind: "study".to_string(),
            title: format!("Added study card: {front}"),
            body: Some(back),
            deep_link: None,
            source: Some("create_study_card".to_string()),
        };
        ctx.db.insert_feed(&item)?;

        Ok(format!("Created study card (id {}).", card.id))
    }
}

/// Reviews a due study card, advancing its schedule via SM-2.
pub struct ReviewStudyCard;

#[async_trait::async_trait]
impl Tool for ReviewStudyCard {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "review_study_card".to_string(),
            description: "Record a review of a study card, applying the SM-2 spaced-repetition \
                algorithm to reschedule it. This mutates the user's study deck, so only use it \
                when the user has clearly reported reviewing a specific card."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The card's id."
                    },
                    "quality": {
                        "type": "integer",
                        "description": "Review quality, 0 (total blackout) to 5 (perfect recall).",
                        "minimum": 0,
                        "maximum": 5
                    }
                },
                "required": ["id", "quality"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let id = required_str(args, "id")?;
        let quality = required_quality(args, "quality")?;

        let card = study::review_study_card(ctx.db, id, quality)?;

        let item = QuietFeedItem {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            kind: "study".to_string(),
            title: format!("Reviewed card {}", card.id),
            body: Some(format!("quality={quality} next due {}", card.due_at)),
            deep_link: None,
            source: Some("review_study_card".to_string()),
        };
        ctx.db.insert_feed(&item)?;

        Ok(format!(
            "Reviewed card {}: next due {} (interval {} days).",
            card.id, card.due_at, card.interval_days
        ))
    }
}

/// Read-only: study cards due right now, capped at `MAX_DUE_CARDS`.
pub struct GetDueCards;

#[async_trait::async_trait]
impl Tool for GetDueCards {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "get_due_cards".to_string(),
            description: "Get the user's spaced-repetition study cards that are due for review \
                right now. Use this before answering any question about what's due for review or \
                planning a study session."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": [],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, _args: &Value) -> Result<String> {
        let now = chrono::Utc::now().to_rfc3339();
        let due = ctx.db.due_cards(&now)?;

        if due.is_empty() {
            return Ok("No cards due right now.".to_string());
        }

        let total = due.len();
        let shown = due.iter().take(MAX_DUE_CARDS);
        let mut lines: Vec<String> = shown
            .map(|card| {
                format!(
                    "- [{}] {} (repetitions={}, due {})",
                    card.id, card.front, card.repetitions, card.due_at
                )
            })
            .collect();

        if total > MAX_DUE_CARDS {
            lines.push(format!("...and {} more", total - MAX_DUE_CARDS));
        }

        Ok(format!("{total} card(s) due:\n{}", lines.join("\n")))
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
    async fn create_study_card_inserts_card_and_feed_row() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = CreateStudyCard;
        let args = serde_json::json!({"front": "What is RRF?", "back": "Reciprocal Rank Fusion"});
        let result = tool.execute(&ctx, &args).await.unwrap();
        assert!(result.contains("Created study card"));

        let feed = db.recent_feed(10).unwrap();
        assert_eq!(feed.len(), 1);
        assert_eq!(feed[0].source.as_deref(), Some("create_study_card"));
    }

    #[tokio::test]
    async fn review_study_card_reschedules_and_errors_on_missing() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let create = CreateStudyCard;
        let create_args = serde_json::json!({"front": "front", "back": "back"});
        create.execute(&ctx, &create_args).await.unwrap();
        let card = db.due_cards(&chrono::Utc::now().to_rfc3339()).unwrap().remove(0);

        let review = ReviewStudyCard;
        let review_args = serde_json::json!({"id": card.id, "quality": 5});
        let result = review.execute(&ctx, &review_args).await.unwrap();
        assert!(result.contains(&card.id));

        let feed = db.recent_feed(10).unwrap();
        assert_eq!(feed.len(), 2);
        assert_eq!(feed[0].source.as_deref(), Some("review_study_card"));

        let missing_args = serde_json::json!({"id": "nonexistent", "quality": 5});
        assert!(review.execute(&ctx, &missing_args).await.is_err());
    }

    #[tokio::test]
    async fn get_due_cards_lists_due_cards() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let create = CreateStudyCard;
        let create_args = serde_json::json!({"front": "What is RRF?", "back": "Reciprocal Rank Fusion"});
        create.execute(&ctx, &create_args).await.unwrap();

        let tool = GetDueCards;
        let result = tool.execute(&ctx, &serde_json::json!({})).await.unwrap();

        assert!(result.contains("1 card(s) due"), "got: {result}");
        assert!(result.contains("What is RRF?"), "got: {result}");
        assert!(result.contains("repetitions=0"), "got: {result}");
    }

    #[tokio::test]
    async fn get_due_cards_reports_none_when_empty() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = GetDueCards;
        let result = tool.execute(&ctx, &serde_json::json!({})).await.unwrap();
        assert_eq!(result, "No cards due right now.");
    }

    #[tokio::test]
    async fn get_due_cards_caps_at_twenty_with_suffix() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let create = CreateStudyCard;
        for i in 0..25 {
            let args = serde_json::json!({"front": format!("front {i}"), "back": "back"});
            create.execute(&ctx, &args).await.unwrap();
        }

        let tool = GetDueCards;
        let result = tool.execute(&ctx, &serde_json::json!({})).await.unwrap();
        assert!(result.starts_with("25 card(s) due"), "got: {result}");
        assert!(result.contains("...and 5 more"), "got: {result}");
    }
}
