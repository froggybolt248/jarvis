# ADR 0001: Absolute icon paths in tauri.conf.json (temporary, machine-local)

**Status:** accepted (temporary) · 2026-07-08

## Context

This repo currently lives under a path containing an apostrophe
(`...\Aahaan's Stuff\Projects\jarvis`). `tauri-winres` escapes that apostrophe
incorrectly when generating `resource.rc`, so RC.EXE fails with RC2135
("file not found") on the icon path, breaking every Windows build. Junctions
don't help because tauri-build canonicalizes icon paths, resolving back to the
real (apostrophe) path.

## Decision

Bundle icons are referenced by absolute paths under `C:/dev/jarvis-assets/icons/`
(a clean path outside the repo, populated by copying `src-tauri/icons/`). This
unblocks all local builds. The `src-tauri/icons/` folder remains in the repo as
the source of truth.

## Consequences / exit plan

- These absolute paths are wrong for every other machine. **Before the first
  public release (M10), this must be reverted** to relative `icons/...` paths.
  Contributors on normal paths are unaffected by the underlying bug.
- Long-term options: move the repo to an apostrophe-free path, or wait for the
  upstream tauri-winres escaping fix.
