//! Classification of reclaimable directory kinds.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The category of a reclaimable directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    /// Rust build artifact directory (`target/` beside a `Cargo.toml` or
    /// containing `.rustc_info.json` / `CARGO_OK`).
    RustTarget,
    /// Node.js dependency directory (`node_modules/`).
    NodeModules,
    /// Python virtual environment (`.venv/`).
    Venv,
    /// Python bytecode cache (`__pycache__/`).
    Pycache,
    /// Cargo registry or git checkout cache.
    CargoCache,
    /// Top-level child of `~/.cache` over a size floor.
    CacheChild,
}

/// A single reclaimable directory found during a survey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyEntry {
    /// Absolute path to the reclaimable directory.
    pub path: PathBuf,
    /// Classification of why this directory is reclaimable.
    pub kind: EntryKind,
    /// Total apparent size in bytes (sum of file sizes, not block count).
    pub bytes: u64,
    /// Number of filesystem entries (files + dirs) within the subtree.
    pub entries: u64,
    /// RFC 3339 timestamp of the most-recently-modified file within the subtree.
    pub mtime: DateTime<Utc>,
    /// Age in days relative to the caller-supplied `now`.
    pub age_days: i64,
    /// Name of the owning crate or directory (the immediate parent's name).
    pub crate_name: String,
}
