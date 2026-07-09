// File watcher for the vault: notify-backed, debounced, markdown-only.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use super::Vault;

const DEBOUNCE: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// What kind of filesystem change produced a [`VaultEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultEventKind {
    Created,
    Modified,
    Removed,
}

/// A debounced change to a markdown file inside the vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultEvent {
    pub path: PathBuf,
    pub kind: VaultEventKind,
}

/// True if `path` is a real markdown note worth surfacing: `.md` extension,
/// not a dotfile, not a temp file (e.g. the `.tmp` files `Vault::write`
/// creates while doing an atomic rename).
fn is_relevant(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if file_name.starts_with('.') {
        return false;
    }
    if file_name.contains(".tmp") {
        return false;
    }
    path.extension()
        .map(|e| e.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

/// Collapses bursts of filesystem events for the same path into a single
/// event once things settle. Kept separate from the notify callback / tokio
/// plumbing so the debounce behavior itself can be unit tested without any
/// real waiting.
struct Debouncer {
    pending: HashMap<PathBuf, (Instant, VaultEventKind)>,
    delay: Duration,
}

impl Debouncer {
    fn new(delay: Duration) -> Self {
        Debouncer {
            pending: HashMap::new(),
            delay,
        }
    }

    /// Record (or overwrite) a pending event for `path`, resetting its
    /// quiet-period clock.
    fn record_at(&mut self, path: PathBuf, kind: VaultEventKind, at: Instant) {
        self.pending.insert(path, (at, kind));
    }

    /// Remove and return every pending event whose quiet period has elapsed
    /// as of `now`.
    fn drain_ready_at(&mut self, now: Instant) -> Vec<VaultEvent> {
        let ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (t, _))| now.saturating_duration_since(*t) >= self.delay)
            .map(|(p, _)| p.clone())
            .collect();

        ready
            .into_iter()
            .filter_map(|path| {
                self.pending
                    .remove(&path)
                    .map(|(_, kind)| VaultEvent { path, kind })
            })
            .collect()
    }
}

impl Vault {
    /// Start watching the vault root recursively for markdown changes.
    /// Returns the watcher (must be kept alive for as long as watching
    /// should continue) and a channel of debounced [`VaultEvent`]s.
    pub fn watch(&self) -> Result<(RecommendedWatcher, mpsc::Receiver<VaultEvent>)> {
        let (tx, rx) = mpsc::channel::<VaultEvent>(256);
        let debouncer = Arc::new(Mutex::new(Debouncer::new(DEBOUNCE)));
        let debouncer_cb = debouncer.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            let kind = match event.kind {
                EventKind::Create(_) => VaultEventKind::Created,
                EventKind::Modify(_) => VaultEventKind::Modified,
                EventKind::Remove(_) => VaultEventKind::Removed,
                _ => return,
            };
            let now = Instant::now();
            let mut debouncer = debouncer_cb.lock().expect("debouncer mutex poisoned");
            for path in event.paths {
                if is_relevant(&path) {
                    debouncer.record_at(path, kind, now);
                }
            }
        })
        .context("creating recommended file watcher")?;

        watcher
            .watch(&self.root, RecursiveMode::Recursive)
            .with_context(|| format!("watching vault root {}", self.root.display()))?;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(POLL_INTERVAL).await;
                let ready = {
                    let mut debouncer = debouncer.lock().expect("debouncer mutex poisoned");
                    debouncer.drain_ready_at(Instant::now())
                };
                for event in ready {
                    if tx.send(event).await.is_err() {
                        // Receiver dropped; stop polling.
                        return;
                    }
                }
            }
        });

        Ok((watcher, rx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_relevant_filters_non_markdown_and_temp_files() {
        assert!(is_relevant(Path::new("Daily/2026-07-09.md")));
        assert!(!is_relevant(Path::new("Daily/2026-07-09.txt")));
        assert!(!is_relevant(Path::new(".hidden.md")));
        assert!(!is_relevant(Path::new(".note.md.abc123.tmp")));
    }

    #[test]
    fn is_relevant_is_case_insensitive_for_extension() {
        assert!(is_relevant(Path::new("Study/notes.MD")));
    }

    #[test]
    fn debouncer_collapses_burst_to_latest_kind() {
        let mut d = Debouncer::new(Duration::from_millis(500));
        let base = Instant::now();
        let path = PathBuf::from("Knowledge/note.md");

        d.record_at(path.clone(), VaultEventKind::Created, base);
        d.record_at(path.clone(), VaultEventKind::Modified, base);

        // Not ready yet: only just recorded.
        assert!(d.drain_ready_at(base).is_empty());

        // Ready once delay has elapsed.
        let ready = d.drain_ready_at(base + Duration::from_millis(500));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].path, path);
        assert_eq!(ready[0].kind, VaultEventKind::Modified);

        // Drained, so a second drain finds nothing.
        assert!(d.drain_ready_at(base + Duration::from_secs(1)).is_empty());
    }

    #[test]
    fn debouncer_keeps_distinct_paths_separate() {
        let mut d = Debouncer::new(Duration::from_millis(500));
        let now = Instant::now();
        d.record_at(PathBuf::from("a.md"), VaultEventKind::Created, now);
        d.record_at(
            PathBuf::from("b.md"),
            VaultEventKind::Removed,
            now + Duration::from_millis(400),
        );

        // Only a.md's debounce window has elapsed at t=now+500ms.
        let ready = d.drain_ready_at(now + Duration::from_millis(500));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].path, PathBuf::from("a.md"));

        // b.md becomes ready 500ms after its own record time.
        let ready = d.drain_ready_at(now + Duration::from_millis(900));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].path, PathBuf::from("b.md"));
    }
}
