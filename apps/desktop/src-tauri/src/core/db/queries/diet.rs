use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DietLog {
    pub id: String,
    pub logged_at: String,
    pub description: String,
    pub calories: Option<i64>,
    pub protein_g: Option<i64>,
    pub carbs_g: Option<i64>,
    pub fat_g: Option<i64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DietTargets {
    pub id: String,
    pub effective_date: String,
    pub calories: Option<i64>,
    pub protein_g: Option<i64>,
    pub carbs_g: Option<i64>,
    pub fat_g: Option<i64>,
    pub created_at: String,
}

fn row_to_log(r: &rusqlite::Row) -> rusqlite::Result<DietLog> {
    Ok(DietLog {
        id: r.get(0)?,
        logged_at: r.get(1)?,
        description: r.get(2)?,
        calories: r.get(3)?,
        protein_g: r.get(4)?,
        carbs_g: r.get(5)?,
        fat_g: r.get(6)?,
        confidence: r.get(7)?,
    })
}

fn row_to_targets(r: &rusqlite::Row) -> rusqlite::Result<DietTargets> {
    Ok(DietTargets {
        id: r.get(0)?,
        effective_date: r.get(1)?,
        calories: r.get(2)?,
        protein_g: r.get(3)?,
        carbs_g: r.get(4)?,
        fat_g: r.get(5)?,
        created_at: r.get(6)?,
    })
}

impl Db {
    pub fn insert_log(&self, log: &DietLog) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO diet_logs (id, logged_at, description, calories, protein_g, carbs_g, fat_g, confidence) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    log.id,
                    log.logged_at,
                    log.description,
                    log.calories,
                    log.protein_g,
                    log.carbs_g,
                    log.fat_g,
                    log.confidence,
                ],
            )?;
            Ok(())
        })
    }

    /// Logs for a given calendar date, matched by the `YYYY-MM-DD` prefix of
    /// `logged_at` (an RFC3339 timestamp).
    pub fn logs_for_date(&self, date: &str) -> Result<Vec<DietLog>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, logged_at, description, calories, protein_g, carbs_g, fat_g, confidence \
                 FROM diet_logs WHERE logged_at LIKE ?1 ORDER BY logged_at ASC",
            )?;
            let pattern = format!("{date}%");
            let rows = stmt
                .query_map(params![pattern], row_to_log)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    /// The most recently-effective diet targets (as of now).
    pub fn current_targets(&self) -> Result<Option<DietTargets>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, effective_date, calories, protein_g, carbs_g, fat_g, created_at \
                 FROM diet_targets ORDER BY effective_date DESC LIMIT 1",
                [],
                row_to_targets,
            )
            .optional()
            .map_err(Into::into)
        })
    }

    pub fn set_targets(&self, targets: &DietTargets) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO diet_targets (id, effective_date, calories, protein_g, carbs_g, fat_g, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    targets.id,
                    targets.effective_date,
                    targets.calories,
                    targets.protein_g,
                    targets.carbs_g,
                    targets.fat_g,
                    targets.created_at,
                ],
            )?;
            Ok(())
        })
    }
}

impl DietLog {
    pub fn new(
        logged_at: impl Into<String>,
        description: impl Into<String>,
        calories: Option<i64>,
        protein_g: Option<i64>,
        carbs_g: Option<i64>,
        fat_g: Option<i64>,
        confidence: Option<f64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            logged_at: logged_at.into(),
            description: description.into(),
            calories,
            protein_g,
            carbs_g,
            fat_g,
            confidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diet_log_round_trip() {
        let db = Db::open_in_memory().unwrap();
        let log = DietLog::new(
            "2026-07-09T12:30:00Z",
            "chicken and rice",
            Some(600),
            Some(45),
            Some(60),
            Some(15),
            Some(0.9),
        );
        db.insert_log(&log).unwrap();

        let logs = db.logs_for_date("2026-07-09").unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], log);

        let empty = db.logs_for_date("2026-07-08").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn diet_targets_round_trip() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.current_targets().unwrap().is_none());

        let targets = DietTargets {
            id: Uuid::new_v4().to_string(),
            effective_date: "2026-07-01".into(),
            calories: Some(2400),
            protein_g: Some(180),
            carbs_g: Some(250),
            fat_g: Some(70),
            created_at: "2026-07-01T00:00:00Z".into(),
        };
        db.set_targets(&targets).unwrap();

        let fetched = db.current_targets().unwrap().unwrap();
        assert_eq!(fetched, targets);
    }
}
