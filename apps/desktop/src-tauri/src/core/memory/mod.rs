// Owned by WP-Vault: markdown vault read/write/watch.

pub mod watcher;

use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use uuid::Uuid;

pub use watcher::{VaultEvent, VaultEventKind};

// The vault is seeded from `vault-template/` at the repo root on first run.
// Contents are embedded at compile time so the running app never depends on
// that directory existing on disk.
const PROFILE_MD: &str = include_str!("../../../../../../vault-template/00-Core/profile.md");
const CORE_MEMORY_MD: &str =
    include_str!("../../../../../../vault-template/00-Core/core-memory.md");
const WELCOME_MD: &str = include_str!("../../../../../../vault-template/Knowledge/welcome.md");

const SEED_DIRS: &[&str] = &["00-Core", "Daily", "Knowledge", "Diet", "Gym", "Study"];
const SEED_FILES: &[(&str, &str)] = &[
    ("00-Core/profile.md", PROFILE_MD),
    ("00-Core/core-memory.md", CORE_MEMORY_MD),
    ("Knowledge/welcome.md", WELCOME_MD),
];

/// A markdown-file-backed vault: the user's editable, Obsidian-compatible
/// source of truth. All paths passed to its methods are relative to the
/// vault root and are validated to stay inside it.
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    /// Open (creating if necessary) the vault rooted at `root`, seeding it
    /// from the bundled template on first run. Never overwrites existing
    /// files.
    pub fn open(root: &Path) -> Result<Vault> {
        fs::create_dir_all(root)
            .with_context(|| format!("creating vault root at {}", root.display()))?;
        let vault = Vault {
            root: root.to_path_buf(),
        };
        vault.seed_template()?;
        Ok(vault)
    }

    /// Root directory of this vault.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn seed_template(&self) -> Result<()> {
        for dir in SEED_DIRS {
            fs::create_dir_all(self.root.join(dir))
                .with_context(|| format!("creating vault skeleton dir {dir}"))?;
        }
        for (rel, content) in SEED_FILES {
            let target = self.safe_join(rel)?;
            if !target.exists() {
                self.write(rel, content)
                    .with_context(|| format!("seeding {rel}"))?;
            }
        }
        Ok(())
    }

    /// Resolve `rel` against the vault root, rejecting anything that would
    /// escape it (absolute paths, `..` that climbs above root). Purely
    /// lexical: never touches the filesystem, so a rejected path can never
    /// cause a read/write outside the vault.
    fn safe_join(&self, rel: &str) -> Result<PathBuf> {
        let rel_path = Path::new(rel);
        let mut out = PathBuf::new();
        let mut depth: i32 = 0;
        for comp in rel_path.components() {
            match comp {
                Component::Normal(part) => {
                    out.push(part);
                    depth += 1;
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    depth -= 1;
                    if depth < 0 {
                        bail!("path escapes vault root: {rel}");
                    }
                    out.pop();
                }
                Component::RootDir | Component::Prefix(_) => {
                    bail!("absolute paths are not allowed: {rel}");
                }
            }
        }
        Ok(self.root.join(out))
    }

    /// Read a note's full contents.
    pub fn read(&self, rel: &str) -> Result<String> {
        let target = self.safe_join(rel)?;
        fs::read_to_string(&target).with_context(|| format!("reading {}", target.display()))
    }

    /// Atomically write `content` to `rel`: writes to a temp file in the
    /// same directory, then renames over the target so a crash never leaves
    /// a half-written note. Creates parent directories as needed.
    pub fn write(&self, rel: &str, content: &str) -> Result<()> {
        let target = self.safe_join(rel)?;
        let parent = target
            .parent()
            .ok_or_else(|| anyhow!("target path has no parent: {}", target.display()))?;
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent dir {}", parent.display()))?;

        let file_name = target
            .file_name()
            .ok_or_else(|| anyhow!("target path has no file name: {}", target.display()))?
            .to_string_lossy();
        let tmp_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));

        fs::write(&tmp_path, content)
            .with_context(|| format!("writing temp file {}", tmp_path.display()))?;
        fs::rename(&tmp_path, &target).with_context(|| {
            format!(
                "renaming {} -> {}",
                tmp_path.display(),
                target.display()
            )
        })?;
        Ok(())
    }

    /// Append `body` under a `## heading` section in `rel`. If the heading
    /// already exists, `body` is inserted at the end of that section
    /// (before the next `##` heading or EOF), leaving the rest of the file
    /// untouched. If the heading is absent, a new section is appended at
    /// the end of the file. Creates the file if it doesn't exist.
    pub fn append_section(&self, rel: &str, heading: &str, body: &str) -> Result<()> {
        let existing = self.read(rel).unwrap_or_default();
        let heading_line = format!("## {heading}");
        let mut lines: Vec<String> = if existing.is_empty() {
            Vec::new()
        } else {
            existing.lines().map(|l| l.to_string()).collect()
        };

        if let Some(idx) = lines.iter().position(|l| l.trim_end() == heading_line) {
            // Section end = next line starting a new heading, or EOF.
            let mut end = lines.len();
            for (j, line) in lines.iter().enumerate().skip(idx + 1) {
                if line.starts_with("## ") {
                    end = j;
                    break;
                }
            }
            // Trim trailing blank lines within the section before inserting.
            let mut insert_at = end;
            while insert_at > idx + 1 && lines[insert_at - 1].trim().is_empty() {
                insert_at -= 1;
            }
            let body_lines: Vec<String> = body.trim_end().lines().map(|l| l.to_string()).collect();
            for (offset, line) in body_lines.into_iter().enumerate() {
                lines.insert(insert_at + offset, line);
            }
        } else {
            if !lines.is_empty() && !lines.last().unwrap().trim().is_empty() {
                lines.push(String::new());
            }
            lines.push(heading_line);
            lines.push(String::new());
            for line in body.trim_end().lines() {
                lines.push(line.to_string());
            }
        }

        let mut new_content = lines.join("\n");
        new_content.push('\n');
        self.write(rel, &new_content)
    }

    /// Relative path of the daily note for `date`, e.g. `Daily/2026-07-09.md`.
    pub fn daily_note_path(&self, date: chrono::NaiveDate) -> String {
        format!("Daily/{}.md", date.format("%Y-%m-%d"))
    }

    /// Append `body` under `## heading` in the daily note for `date`,
    /// creating the note if needed.
    pub fn append_to_daily(
        &self,
        date: chrono::NaiveDate,
        heading: &str,
        body: &str,
    ) -> Result<()> {
        let rel = self.daily_note_path(date);
        self.append_section(&rel, heading, body)
    }

    /// Recursively list every `.md` file in the vault, relative to root.
    pub fn list_markdown(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        Self::walk_markdown(&self.root, &self.root, &mut out)?;
        Ok(out)
    }

    fn walk_markdown(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir).with_context(|| format!("reading dir {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                Self::walk_markdown(root, &path, out)?;
            } else if file_type.is_file() {
                let is_md = path
                    .extension()
                    .map(|e| e.eq_ignore_ascii_case("md"))
                    .unwrap_or(false);
                if is_md {
                    let rel = path.strip_prefix(root)?.to_path_buf();
                    out.push(rel);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn open_seeds_template_on_empty_dir() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();

        assert!(dir.path().join("00-Core/profile.md").exists());
        assert!(dir.path().join("00-Core/core-memory.md").exists());
        assert!(dir.path().join("Knowledge/welcome.md").exists());
        for d in SEED_DIRS {
            assert!(dir.path().join(d).is_dir());
        }

        let profile = vault.read("00-Core/profile.md").unwrap();
        assert_eq!(profile, PROFILE_MD);
    }

    #[test]
    fn open_does_not_overwrite_user_edits() {
        let dir = tempdir().unwrap();
        {
            let vault = Vault::open(dir.path()).unwrap();
            vault
                .write("00-Core/profile.md", "# About Me\n\nName: Aahaan\n")
                .unwrap();
        }
        // Reopen: seeding must not clobber the user's edit.
        let vault2 = Vault::open(dir.path()).unwrap();
        let content = vault2.read("00-Core/profile.md").unwrap();
        assert_eq!(content, "# About Me\n\nName: Aahaan\n");
    }

    #[test]
    fn atomic_write_and_read_round_trip() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        vault.write("Knowledge/note.md", "hello vault").unwrap();
        assert_eq!(vault.read("Knowledge/note.md").unwrap(), "hello vault");
        // No leftover temp files.
        let entries: Vec<_> = fs::read_dir(dir.path().join("Knowledge"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert!(entries.iter().all(|n| !n.contains(".tmp")));
    }

    #[test]
    fn write_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        vault.write("Study/cs/notes.md", "content").unwrap();
        assert_eq!(vault.read("Study/cs/notes.md").unwrap(), "content");
    }

    #[test]
    fn append_section_appends_under_existing_heading() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        vault
            .write(
                "Daily/2026-07-09.md",
                "# 2026-07-09\n\n## Meals\n\n- Oatmeal\n\n## Workouts\n\n- Run 5k\n",
            )
            .unwrap();

        vault
            .append_to_daily(
                chrono::NaiveDate::from_ymd_opt(2026, 7, 9).unwrap(),
                "Meals",
                "- Chicken salad",
            )
            .unwrap();

        let content = vault.read("Daily/2026-07-09.md").unwrap();
        assert!(content.contains("- Oatmeal"));
        assert!(content.contains("- Chicken salad"));
        // The other section stays intact and after Meals.
        let meals_idx = content.find("## Meals").unwrap();
        let workouts_idx = content.find("## Workouts").unwrap();
        let chicken_idx = content.find("- Chicken salad").unwrap();
        assert!(meals_idx < chicken_idx);
        assert!(chicken_idx < workouts_idx);
        assert!(content.contains("- Run 5k"));
    }

    #[test]
    fn append_section_creates_heading_when_absent() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        vault
            .write("Daily/2026-07-09.md", "# 2026-07-09\n\n## Meals\n\n- Oatmeal\n")
            .unwrap();

        vault
            .append_to_daily(
                chrono::NaiveDate::from_ymd_opt(2026, 7, 9).unwrap(),
                "Workouts",
                "- Run 5k",
            )
            .unwrap();

        let content = vault.read("Daily/2026-07-09.md").unwrap();
        assert!(content.contains("## Meals"));
        assert!(content.contains("- Oatmeal"));
        assert!(content.contains("## Workouts"));
        assert!(content.contains("- Run 5k"));
        assert!(content.find("## Meals").unwrap() < content.find("## Workouts").unwrap());
    }

    #[test]
    fn append_section_on_missing_file_creates_it() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        vault
            .append_section("Knowledge/fresh.md", "Notes", "first line")
            .unwrap();
        let content = vault.read("Knowledge/fresh.md").unwrap();
        assert!(content.contains("## Notes"));
        assert!(content.contains("first line"));
    }

    #[test]
    fn rejects_relative_parent_traversal() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();

        assert!(vault.read("../../evil.txt").is_err());
        assert!(vault.write("../../evil.txt", "pwned").is_err());

        // Confirm nothing was written outside the vault root.
        let outside = dir.path().parent().unwrap().parent().unwrap().join("evil.txt");
        assert!(!outside.exists());
    }

    #[test]
    fn rejects_absolute_paths() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();

        #[cfg(windows)]
        let abs = "C:\\Windows\\evil.txt";
        #[cfg(not(windows))]
        let abs = "/etc/evil.txt";

        assert!(vault.read(abs).is_err());
        assert!(vault.write(abs, "pwned").is_err());
    }

    #[test]
    fn rejects_dotdot_that_climbs_above_root_even_mid_path() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();

        assert!(vault.read("Study/../../secret.md").is_err());
        assert!(vault.write("Study/../../secret.md", "pwned").is_err());

        let outside = dir.path().parent().unwrap().join("secret.md");
        assert!(!outside.exists());
    }

    #[test]
    fn dotdot_within_root_is_fine() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        // Study/../Knowledge/note.md normalizes to Knowledge/note.md, which
        // stays inside the vault root, so this must succeed.
        vault
            .write("Study/../Knowledge/note.md", "still inside")
            .unwrap();
        assert_eq!(
            vault.read("Knowledge/note.md").unwrap(),
            "still inside"
        );
    }

    #[test]
    fn list_markdown_finds_nested_notes() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        vault.write("Study/cs/notes.md", "a").unwrap();
        vault.write("Daily/2026-07-09.md", "b").unwrap();
        vault.write("Study/cs/data.txt", "not markdown").unwrap();

        let mut found = vault.list_markdown().unwrap();
        found.sort();

        let mut found_strs: Vec<String> = found
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        found_strs.sort();

        assert!(found_strs.contains(&"Study/cs/notes.md".to_string()));
        assert!(found_strs.contains(&"Daily/2026-07-09.md".to_string()));
        assert!(found_strs.contains(&"00-Core/profile.md".to_string()));
        assert!(!found_strs.iter().any(|p| p.ends_with("data.txt")));
    }
}
