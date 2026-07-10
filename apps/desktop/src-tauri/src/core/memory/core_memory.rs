// WP-Memory-RAG owns this file.
//! Renders core-memory entries (durable facts about the user, pinned or
//! not) into a compact block suitable for inclusion in a system prompt.

use anyhow::Result;

use crate::core::db::queries::core_memory::CoreMemoryEntry;
use crate::core::db::Db;

/// Render core-memory entries into a compact block for the system prompt,
/// pinned first, as `"- {label}: {content}"` lines under a short header.
/// Empty string if none.
pub fn render_core_memory(entries: &[CoreMemoryEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    // Partition by pinned, keeping each group's relative input order (the DB
    // query already orders by pinned DESC, updated_at DESC).
    let pinned = entries.iter().filter(|e| e.pinned);
    let unpinned = entries.iter().filter(|e| !e.pinned);

    let mut out = String::from("Core memory:\n");
    for entry in pinned.chain(unpinned) {
        out.push_str(&format!("- {}: {}\n", entry.label, entry.content));
    }
    out.truncate(out.trim_end_matches('\n').len());
    out
}

/// Convenience: load from the DB and render.
pub fn load_and_render(db: &Db) -> Result<String> {
    let entries = db.list_core_memory()?;
    Ok(render_core_memory(&entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, label: &str, content: &str, pinned: bool) -> CoreMemoryEntry {
        CoreMemoryEntry {
            id: id.to_string(),
            label: label.to_string(),
            content: content.to_string(),
            pinned,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn empty_entries_render_empty_string() {
        assert_eq!(render_core_memory(&[]), String::new());
    }

    #[test]
    fn pinned_entries_come_first() {
        let entries = vec![
            entry("1", "Name", "Aahaan", false),
            entry("2", "Timezone", "PT", true),
            entry("3", "Diet", "vegetarian", false),
            entry("4", "Priority", "sleep schedule", true),
        ];
        let rendered = render_core_memory(&entries);

        assert!(rendered.starts_with("Core memory:"));
        let timezone_idx = rendered.find("Timezone").unwrap();
        let priority_idx = rendered.find("Priority").unwrap();
        let name_idx = rendered.find("Name").unwrap();
        let diet_idx = rendered.find("Diet").unwrap();

        assert!(timezone_idx < name_idx);
        assert!(timezone_idx < diet_idx);
        assert!(priority_idx < name_idx);
        assert!(priority_idx < diet_idx);

        assert!(rendered.contains("- Name: Aahaan"));
        assert!(rendered.contains("- Timezone: PT"));
    }

    #[test]
    fn load_and_render_reads_from_db() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_core_memory("1", "Name", "Aahaan", true, "2026-01-01T00:00:00Z")
            .unwrap();
        db.upsert_core_memory("2", "Diet", "vegetarian", false, "2026-01-01T00:00:00Z")
            .unwrap();

        let rendered = load_and_render(&db).unwrap();
        assert!(rendered.contains("- Name: Aahaan"));
        assert!(rendered.contains("- Diet: vegetarian"));
    }

    #[test]
    fn load_and_render_empty_db_yields_empty_string() {
        let db = Db::open_in_memory().unwrap();
        let rendered = load_and_render(&db).unwrap();
        assert_eq!(rendered, String::new());
    }
}
