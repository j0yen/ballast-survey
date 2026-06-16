# Changelog

## v0.2.0 — 2026-06-16

Cloud-aware fossil classification: cross-references installed binaries against
target/ mtime to flag fossil targets (safe to reclaim). New `cloud_info` field
on RustTarget entries with `reap_safety` rank. `--no-cloudaware` preserves v0.1
schema. Motivating case: wintermute-brain/target (13G) safely reclaimable.
