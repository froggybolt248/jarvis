// WP-Agent-Tools owns this file.

//! Diet-logging agent tools: `log_meal` and `set_diet_targets`. Both mutate
//! the local SQLite diet tables (via the shared `queries::diet` helpers also
//! used by the quick-add form commands) and log a Quiet Feed audit row.

use anyhow::Result;
use serde_json::Value;

use crate::core::agent::provider::ToolDef;
use crate::core::db::queries::diet;
use crate::core::db::queries::quiet_feed::QuietFeedItem;

use super::{Tool, ToolContext};

/// Extract a required string field from a JSON object.
fn required_str<'a>(args: &'a Value, field: &str) -> Result<&'a str> {
    args.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing or non-string required field '{field}'"))
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

/// Logs a meal the user ate, with optional macro estimates.
pub struct LogMeal;

#[async_trait::async_trait]
impl Tool for LogMeal {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "log_meal".to_string(),
            description: "Log a meal the user ate, with optional calorie/macro estimates. This \
                mutates the user's diet log, so only use it when the user has clearly described \
                something they ate."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "description": {
                        "type": "string",
                        "description": "What was eaten, e.g. 'chicken and rice'."
                    },
                    "calories": {
                        "type": "integer",
                        "description": "Estimated calories."
                    },
                    "protein_g": {
                        "type": "integer",
                        "description": "Estimated protein in grams."
                    },
                    "carbs_g": {
                        "type": "integer",
                        "description": "Estimated carbs in grams."
                    },
                    "fat_g": {
                        "type": "integer",
                        "description": "Estimated fat in grams."
                    }
                },
                "required": ["description"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let description = required_str(args, "description")?.to_string();
        let calories = optional_i64(args, "calories")?;
        let protein_g = optional_i64(args, "protein_g")?;
        let carbs_g = optional_i64(args, "carbs_g")?;
        let fat_g = optional_i64(args, "fat_g")?;

        let log = diet::log_meal(ctx.db, description.clone(), calories, protein_g, carbs_g, fat_g)?;

        let item = QuietFeedItem {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            kind: "diet".to_string(),
            title: format!("Logged meal: {description}"),
            body: calories.map(|c| format!("{c} kcal")),
            deep_link: None,
            source: Some("log_meal".to_string()),
        };
        ctx.db.insert_feed(&item)?;

        Ok(format!("Logged meal '{description}' (id {}).", log.id))
    }
}

/// Sets the user's daily diet targets, effective today.
pub struct SetDietTargets;

#[async_trait::async_trait]
impl Tool for SetDietTargets {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "set_diet_targets".to_string(),
            description: "Set the user's daily diet targets (calories and/or macros), effective \
                today. This mutates the user's diet targets, so only use it when the user has \
                clearly asked to set or change a target. At least one field must be provided."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "calories": {
                        "type": "integer",
                        "description": "Daily calorie target."
                    },
                    "protein_g": {
                        "type": "integer",
                        "description": "Daily protein target in grams."
                    },
                    "carbs_g": {
                        "type": "integer",
                        "description": "Daily carbs target in grams."
                    },
                    "fat_g": {
                        "type": "integer",
                        "description": "Daily fat target in grams."
                    }
                },
                "required": [],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let calories = optional_i64(args, "calories")?;
        let protein_g = optional_i64(args, "protein_g")?;
        let carbs_g = optional_i64(args, "carbs_g")?;
        let fat_g = optional_i64(args, "fat_g")?;

        let targets = diet::set_diet_targets(ctx.db, calories, protein_g, carbs_g, fat_g)?;

        let item = QuietFeedItem {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            kind: "diet".to_string(),
            title: "Updated diet targets".to_string(),
            body: Some(format!(
                "calories={:?} protein_g={:?} carbs_g={:?} fat_g={:?}",
                targets.calories, targets.protein_g, targets.carbs_g, targets.fat_g
            )),
            deep_link: None,
            source: Some("set_diet_targets".to_string()),
        };
        ctx.db.insert_feed(&item)?;

        Ok("Updated diet targets.".to_string())
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

    fn ctx_parts() -> (Db, tempfile::TempDir, Vault, StubProvider) {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        (db, dir, vault, StubProvider)
    }

    #[tokio::test]
    async fn log_meal_inserts_log_and_feed_row() {
        let (db, _dir, vault, provider) = ctx_parts();
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = LogMeal;
        let args = serde_json::json!({"description": "chicken and rice", "calories": 600});
        let result = tool.execute(&ctx, &args).await.unwrap();
        assert!(result.contains("chicken and rice"));

        let feed = db.recent_feed(10).unwrap();
        assert_eq!(feed.len(), 1);
        assert_eq!(feed[0].kind, "diet");
        assert_eq!(feed[0].source.as_deref(), Some("log_meal"));
    }

    #[tokio::test]
    async fn log_meal_missing_description_errors() {
        let (db, _dir, vault, provider) = ctx_parts();
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = LogMeal;
        let args = serde_json::json!({});
        assert!(tool.execute(&ctx, &args).await.is_err());
    }

    #[tokio::test]
    async fn set_diet_targets_with_no_fields_errors() {
        let (db, _dir, vault, provider) = ctx_parts();
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = SetDietTargets;
        let args = serde_json::json!({});
        assert!(tool.execute(&ctx, &args).await.is_err());
    }

    #[tokio::test]
    async fn set_diet_targets_writes_targets_and_feed_row() {
        let (db, _dir, vault, provider) = ctx_parts();
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = SetDietTargets;
        let args = serde_json::json!({"calories": 2200, "protein_g": 170});
        tool.execute(&ctx, &args).await.unwrap();

        let targets = db.current_targets().unwrap().unwrap();
        assert_eq!(targets.calories, Some(2200));
        assert_eq!(targets.protein_g, Some(170));

        let feed = db.recent_feed(10).unwrap();
        assert_eq!(feed.len(), 1);
        assert_eq!(feed[0].source.as_deref(), Some("set_diet_targets"));
    }
}
