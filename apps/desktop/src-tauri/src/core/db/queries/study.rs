use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::core::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SrsCard {
    pub id: String,
    pub course_id: Option<String>,
    pub front: String,
    pub back: String,
    pub ease_factor: f64,
    pub interval_days: i64,
    pub repetitions: i64,
    pub due_at: String,
    pub created_at: String,
}

fn row_to_card(r: &rusqlite::Row) -> rusqlite::Result<SrsCard> {
    Ok(SrsCard {
        id: r.get(0)?,
        course_id: r.get(1)?,
        front: r.get(2)?,
        back: r.get(3)?,
        ease_factor: r.get(4)?,
        interval_days: r.get(5)?,
        repetitions: r.get(6)?,
        due_at: r.get(7)?,
        created_at: r.get(8)?,
    })
}

const SELECT_COLUMNS: &str =
    "id, course_id, front, back, ease_factor, interval_days, repetitions, due_at, created_at";

impl Db {
    pub fn insert_card(&self, card: &SrsCard) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO srs_cards (id, course_id, front, back, ease_factor, interval_days, repetitions, due_at, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    card.id,
                    card.course_id,
                    card.front,
                    card.back,
                    card.ease_factor,
                    card.interval_days,
                    card.repetitions,
                    card.due_at,
                    card.created_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn due_cards(&self, now: &str) -> Result<Vec<SrsCard>> {
        self.with_conn(|conn| {
            let sql =
                format!("SELECT {SELECT_COLUMNS} FROM srs_cards WHERE due_at <= ?1 ORDER BY due_at ASC");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt
                .query_map(params![now], row_to_card)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    pub fn update_card_schedule(
        &self,
        id: &str,
        ease_factor: f64,
        interval_days: i64,
        repetitions: i64,
        due_at: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE srs_cards SET ease_factor = ?2, interval_days = ?3, repetitions = ?4, due_at = ?5 WHERE id = ?1",
                params![id, ease_factor, interval_days, repetitions, due_at],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn srs_card_round_trip_and_scheduling() {
        let db = Db::open_in_memory().unwrap();
        let card = SrsCard {
            id: Uuid::new_v4().to_string(),
            course_id: Some("cs101".into()),
            front: "What is RRF?".into(),
            back: "Reciprocal Rank Fusion".into(),
            ease_factor: 2.5,
            interval_days: 0,
            repetitions: 0,
            due_at: "2026-07-09T00:00:00Z".into(),
            created_at: "2026-07-01T00:00:00Z".into(),
        };
        db.insert_card(&card).unwrap();

        let due = db.due_cards("2026-07-09T12:00:00Z").unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0], card);

        let not_yet_due = db.due_cards("2026-07-01T00:00:00Z").unwrap();
        assert!(not_yet_due.is_empty());

        db.update_card_schedule(&card.id, 2.6, 1, 1, "2026-07-10T00:00:00Z")
            .unwrap();
        let updated = db.due_cards("2026-07-10T00:00:00Z").unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].ease_factor, 2.6);
        assert_eq!(updated[0].interval_days, 1);
        assert_eq!(updated[0].repetitions, 1);
        assert_eq!(updated[0].due_at, "2026-07-10T00:00:00Z");
    }
}
