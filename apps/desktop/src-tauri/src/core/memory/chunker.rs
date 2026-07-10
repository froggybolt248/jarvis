// WP-Memory-RAG owns this file.
//! Heading-aware markdown chunker: splits a note into sections delimited by
//! ATX headings (`#` through `######`). Any content before the first heading
//! becomes a preamble chunk with `heading = None`. Each subsequent chunk
//! begins at a heading line and runs until the next heading of any level
//! (or EOF), including the heading line itself in its content.

/// One chunk extracted from a markdown source file.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkInput {
    pub id: String,
    pub heading: Option<String>,
    pub content: String,
}

/// Returns `Some(heading_text)` if `line` is an ATX heading line (`#`..`######`
/// followed by a space or EOL), else `None`.
fn atx_heading_text(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    // Must be followed by whitespace or end-of-line to count as an ATX heading.
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    Some(rest.trim().to_string())
}

/// Split markdown into heading-delimited sections. See module docs.
pub fn chunk_markdown(source_path: &str, content: &str) -> Vec<ChunkInput> {
    let lines: Vec<&str> = content.lines().collect();

    // Find indices of heading lines.
    let mut heading_indices: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if atx_heading_text(line).is_some() {
            heading_indices.push(i);
        }
    }

    let mut sections: Vec<(Option<String>, String)> = Vec::new();

    let first_heading = heading_indices.first().copied().unwrap_or(lines.len());
    if first_heading > 0 {
        let preamble = lines[0..first_heading].join("\n");
        sections.push((None, preamble));
    }

    for (idx, &start) in heading_indices.iter().enumerate() {
        let end = heading_indices
            .get(idx + 1)
            .copied()
            .unwrap_or(lines.len());
        let heading_text = atx_heading_text(lines[start]).unwrap_or_default();
        let body = lines[start..end].join("\n");
        sections.push((Some(heading_text), body));
    }

    let mut out = Vec::new();
    let mut ordinal = 0usize;
    for (heading, body) in sections {
        if body.trim().is_empty() {
            continue;
        }
        out.push(ChunkInput {
            id: format!("{source_path}#{ordinal}"),
            heading,
            content: body,
        });
        ordinal += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preamble_before_first_heading_has_no_heading() {
        let content = "Some intro text.\nMore intro.\n\n# First Heading\n\nBody text.\n";
        let chunks = chunk_markdown("notes.md", content);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].heading, None);
        assert!(chunks[0].content.contains("Some intro text."));
        assert_eq!(chunks[1].heading.as_deref(), Some("First Heading"));
        assert!(chunks[1].content.contains("# First Heading"));
        assert!(chunks[1].content.contains("Body text."));
    }

    #[test]
    fn no_preamble_when_file_starts_with_heading() {
        let content = "# Title\n\nBody.\n";
        let chunks = chunk_markdown("notes.md", content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].heading.as_deref(), Some("Title"));
    }

    #[test]
    fn multiple_headings_of_various_levels_split_correctly() {
        let content = "\
# Title

Intro paragraph.

## Section A

Content A.

### Subsection A.1

Content A.1.

## Section B

Content B.
";
        let chunks = chunk_markdown("doc.md", content);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].heading.as_deref(), Some("Title"));
        assert!(chunks[0].content.contains("Intro paragraph."));
        assert!(!chunks[0].content.contains("Content A."));

        assert_eq!(chunks[1].heading.as_deref(), Some("Section A"));
        assert!(chunks[1].content.contains("Content A."));
        assert!(!chunks[1].content.contains("Content A.1"));

        assert_eq!(chunks[2].heading.as_deref(), Some("Subsection A.1"));
        assert!(chunks[2].content.contains("Content A.1."));

        assert_eq!(chunks[3].heading.as_deref(), Some("Section B"));
        assert!(chunks[3].content.contains("Content B."));
    }

    #[test]
    fn ids_are_deterministic_and_unique_within_file() {
        let content = "# A\n\nbody a\n\n# B\n\nbody b\n\n# C\n\nbody c\n";
        let chunks1 = chunk_markdown("path/to/file.md", content);
        let chunks2 = chunk_markdown("path/to/file.md", content);
        let ids1: Vec<&str> = chunks1.iter().map(|c| c.id.as_str()).collect();
        let ids2: Vec<&str> = chunks2.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids1, ids2);

        let unique: std::collections::HashSet<&str> = ids1.iter().copied().collect();
        assert_eq!(unique.len(), ids1.len());
        assert_eq!(ids1, vec!["path/to/file.md#0", "path/to/file.md#1", "path/to/file.md#2"]);
    }

    #[test]
    fn whitespace_only_preamble_is_dropped() {
        // A heading-led chunk always includes its heading line, so it's
        // never whitespace-only; only a preamble with no real content can be
        // whitespace-only and dropped.
        let content = "   \n\n# Real Heading\n\nActual content.\n";
        let chunks = chunk_markdown("notes.md", content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].heading.as_deref(), Some("Real Heading"));
    }

    #[test]
    fn whitespace_only_body_content_is_still_kept_because_of_heading_line() {
        // The heading line itself is real content, so a section whose body
        // is blank still yields a (small) non-empty chunk.
        let content = "# Real Heading\n\nActual content.\n\n## Empty Section\n\n   \n\t\n";
        let chunks = chunk_markdown("notes.md", content);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[1].heading.as_deref(), Some("Empty Section"));
    }

    #[test]
    fn entirely_empty_content_yields_no_chunks() {
        let chunks = chunk_markdown("empty.md", "");
        assert!(chunks.is_empty());
    }

    #[test]
    fn heading_without_space_after_hashes_is_not_treated_as_heading() {
        // "#tag" is not a valid ATX heading (no space after hashes), so it
        // should be treated as regular body content, not a chunk boundary.
        let content = "# Title\n\nSome text with #tag inline.\n";
        let chunks = chunk_markdown("notes.md", content);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("#tag inline"));
    }
}
