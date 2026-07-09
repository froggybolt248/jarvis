use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::core::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GymSession {
    pub id: String,
    pub program_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GymSet {
    pub id: String,
    pub session_id: String,
    pub exercise: String,
    pub weight: Option<f64>,
    pub reps: Option<i64>,
    pub rpe: Option<f64>,
    pub set_index: Option<i64>,
}

fn row_to_session(r: &rusqlite::Row) -> rusqlite::Result<GymSession> {
    Ok(GymSession {
        id: r.get(0)?,
        program_id: r.get(1)?,
        started_at: r.get(2)?,
        ended_at: r.get(3)?,
        notes: r.get(4)?,
    })
}

fn row_to_set(r: &rusqlite::Row) -> rusqlite::Result<GymSet> {
    Ok(GymSet {
        id: r.get(0)?,
        session_id: r.get(1)?,
        exercise: r.get(2)?,
        weight: r.get(3)?,
        reps: r.get(4)?,
        rpe: r.get(5)?,
        set_index: r.get(6)?,
    })
}

impl Db {
    pub fn insert_session(&self, session: &GymSession) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO gym_sessions (id, program_id, started_at, ended_at, notes) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    session.id,
                    session.program_id,
                    session.started_at,
                    session.ended_at,
                    session.notes,
                ],
            )?;
            Ok(())
        })
    }

    pub fn insert_set(&self, set: &GymSet) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO gym_sets (id, session_id, exercise, weight, reps, rpe, set_index) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    set.id,
                    set.session_id,
                    set.exercise,
                    set.weight,
                    set.reps,
                    set.rpe,
                    set.set_index,
                ],
            )?;
            Ok(())
        })
    }

    pub fn sets_for_exercise(&self, exercise: &str, limit: usize) -> Result<Vec<GymSet>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT gym_sets.id, gym_sets.session_id, gym_sets.exercise, gym_sets.weight, gym_sets.reps, gym_sets.rpe, gym_sets.set_index \
                 FROM gym_sets JOIN gym_sessions ON gym_sessions.id = gym_sets.session_id \
                 WHERE gym_sets.exercise = ?1 ORDER BY gym_sessions.started_at DESC, gym_sets.set_index ASC LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![exercise, limit as i64], row_to_set)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    pub fn recent_sessions(&self, limit: usize) -> Result<Vec<GymSession>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, program_id, started_at, ended_at, notes FROM gym_sessions ORDER BY started_at DESC LIMIT ?1",
            )?;
            let rows = stmt
                .query_map(params![limit as i64], row_to_session)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }
}
