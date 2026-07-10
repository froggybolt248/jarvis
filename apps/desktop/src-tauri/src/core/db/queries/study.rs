use anyhow::Result;
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

    /// A single card by id, or `None` if it doesn't exist.
    pub fn get_card(&self, id: &str) -> Result<Option<SrsCard>> {
        self.with_conn(|conn| {
            let sql = format!("SELECT {SELECT_COLUMNS} FROM srs_cards WHERE id = ?1");
            conn.query_row(&sql, params![id], row_to_card)
                .optional()
                .map_err(Into::into)
        })
    }
}

/// Applies the canonical SM-2 spaced-repetition algorithm to a card's current
/// scheduling state given a review `quality` (0..=5), returning the new
/// `(ease_factor, interval_days, repetitions)`. Pure and side-effect free;
/// the caller is responsible for computing `due_at` from `interval_days` and
/// persisting the result.
pub(crate) fn apply_sm2(
    ease_factor: f64,
    interval_days: i64,
    repetitions: i64,
    quality: u8,
) -> (f64, i64, i64) {
    let q = quality.min(5) as f64;

    let mut new_ef = ease_factor + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02));
    if new_ef < 1.3 {
        new_ef = 1.3;
    }

    if quality < 3 {
        (new_ef, 1, 0)
    } else {
        let new_reps = repetitions + 1;
        let new_interval = if new_reps == 1 {
            1
        } else if new_reps == 2 {
            6
        } else {
            (interval_days as f64 * new_ef).round() as i64
        };
        (new_ef, new_interval, new_reps)
    }
}

/// Creates a new SRS card, due immediately, with the default SM-2 starting
/// state (ease factor 2.5, interval 0, 0 repetitions).
pub(crate) fn create_study_card(
    db: &Db,
    front: String,
    back: String,
    course_id: Option<String>,
) -> Result<SrsCard> {
    let now = Utc::now().to_rfc3339();
    let card = SrsCard {
        id: Uuid::new_v4().to_string(),
        course_id,
        front,
        back,
        ease_factor: 2.5,
        interval_days: 0,
        repetitions: 0,
        due_at: now.clone(),
        created_at: now,
    };
    db.insert_card(&card)?;
    Ok(card)
}

/// Reviews a card: applies SM-2 to its current scheduling state and persists
/// the update. Errors if no card with `id` exists.
pub(crate) fn review_study_card(db: &Db, id: &str, quality: u8) -> Result<SrsCard> {
    let card = db
        .get_card(id)?
        .ok_or_else(|| anyhow::anyhow!("no such study card: {id}"))?;

    let (ease_factor, interval_days, repetitions) =
        apply_sm2(card.ease_factor, card.interval_days, card.repetitions, quality);
    let due_at = (Utc::now() + Duration::days(interval_days)).to_rfc3339();

    db.update_card_schedule(id, ease_factor, interval_days, repetitions, &due_at)?;

    Ok(SrsCard {
        ease_factor,
        interval_days,
        repetitions,
        due_at,
        ..card
    })
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

    #[test]
    fn get_card_returns_none_when_missing() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.get_card("nonexistent").unwrap().is_none());
    }

    #[test]
    fn get_card_round_trip() {
        let db = Db::open_in_memory().unwrap();
        let card = create_study_card(&db, "front".into(), "back".into(), None).unwrap();
        let fetched = db.get_card(&card.id).unwrap().unwrap();
        assert_eq!(fetched, card);
    }

    #[test]
    fn review_study_card_updates_schedule_and_errors_on_missing() {
        let db = Db::open_in_memory().unwrap();
        let card = create_study_card(&db, "front".into(), "back".into(), None).unwrap();

        let reviewed = review_study_card(&db, &card.id, 5).unwrap();
        assert_eq!(reviewed.repetitions, 1);
        assert_eq!(reviewed.interval_days, 1);

        let fetched = db.get_card(&card.id).unwrap().unwrap();
        assert_eq!(fetched, reviewed);

        assert!(review_study_card(&db, "nonexistent", 5).is_err());
    }

    #[test]
    fn apply_sm2_failing_review_resets_progress() {
        let (ef, interval, reps) = apply_sm2(2.5, 10, 3, 1);
        assert_eq!(interval, 1);
        assert_eq!(reps, 0);
        assert!(ef >= 1.3);
    }

    #[test]
    fn apply_sm2_successive_passes_grow_interval() {
        let (ef1, int1, reps1) = apply_sm2(2.5, 0, 0, 5);
        assert_eq!(reps1, 1);
        assert_eq!(int1, 1);

        let (ef2, int2, reps2) = apply_sm2(ef1, int1, reps1, 5);
        assert_eq!(reps2, 2);
        assert_eq!(int2, 6);

        let (ef3, int3, reps3) = apply_sm2(ef2, int2, reps2, 5);
        assert_eq!(reps3, 3);
        // EF is updated *before* the interval calculation within the same
        // call, so the interval uses this call's own new EF (ef3), not the
        // previous call's (ef2).
        let expected = (int2 as f64 * ef3).round() as i64;
        assert_eq!(int3, expected);
    }

    #[test]
    fn apply_sm2_ease_factor_never_drops_below_1_3() {
        let mut ef = 1.3;
        for _ in 0..20 {
            let (new_ef, _, _) = apply_sm2(ef, 1, 1, 0);
            assert!(new_ef >= 1.3, "ease factor dropped below 1.3: {new_ef}");
            ef = new_ef;
        }
    }
}
