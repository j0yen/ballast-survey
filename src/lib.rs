//! ballast-survey — read-only inventory of reclaimable disk weight.
//!
//! Walks one or more root directories, finds reclaimable subtrees (Rust
//! `target/` dirs, `node_modules/`, `.venv/`, `__pycache__/`, cargo caches,
//! large `~/.cache` children), sizes each, records metadata, and emits
//! structured JSON inventory sorted by reclaimable bytes.
//!
//! **Design invariant:** no wall-clock reads in library code.  All
//! time-dependent logic accepts a caller-supplied `now: DateTime<Utc>`.

pub mod classify;
pub mod cloudaware;
pub mod emit;
pub mod roots;
pub mod size;
pub mod walk;

pub use classify::{EntryKind, SurveyEntry};
pub use cloudaware::BinNameOverrides;
pub use emit::{Output, Summary};
pub use walk::scan_root;

use chrono::{DateTime, Utc};
use std::path::Path;

/// Options for controlling cloud-aware annotation during a survey.
#[derive(Debug, Clone, Default)]
pub struct SurveyOptions {
    /// When `true`, skip cloud-aware annotation entirely.
    ///
    /// The output will match the v0.1 schema exactly — no `cloud_info` fields.
    pub no_cloudaware: bool,
    /// Per-crate binary name overrides: crate dir name → installed binary name.
    ///
    /// Used when the installed binary does not match the crate directory name
    /// (e.g. `"wintermute-brain"` → `"wmd"`).
    pub bin_name_overrides: BinNameOverrides,
}

/// Run a full survey over `roots`, filtering by `min_bytes`.
///
/// `now` is the reference instant used for `age_days` computation — pass the
/// wall-clock value at the CLI boundary, or a fixed instant in tests.
///
/// Equivalent to `survey_with_options` with default options (cloud-aware enabled,
/// no binary name overrides).
#[must_use]
pub fn survey(
    roots: &[impl AsRef<Path>],
    min_bytes: u64,
    now: DateTime<Utc>,
) -> Output {
    survey_with_options(roots, min_bytes, now, &SurveyOptions::default())
}

/// Run a full survey with explicit options controlling cloud-aware annotation.
///
/// Use [`SurveyOptions::no_cloudaware`] to reproduce the v0.1 schema exactly.
#[must_use]
pub fn survey_with_options(
    roots: &[impl AsRef<Path>],
    min_bytes: u64,
    now: DateTime<Utc>,
    opts: &SurveyOptions,
) -> Output {
    let mut entries: Vec<SurveyEntry> = Vec::new();

    for root in roots {
        let root = root.as_ref();
        let mut found = walk::scan_root(root, now);
        entries.append(&mut found);
    }

    // Annotate RustTarget entries with cloud-aware fossil info.
    if !opts.no_cloudaware {
        cloudaware::annotate_entries(&mut entries, &opts.bin_name_overrides);
    }

    // Sort by bytes descending
    entries.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    // Apply min-size filter
    if min_bytes > 0 {
        entries.retain(|e| e.bytes >= min_bytes);
    }

    let reclaimable_bytes: u64 = entries.iter().map(|e| e.bytes).sum();

    Output {
        summary: Summary {
            reclaimable_bytes,
            entry_count: entries.len(),
            scanned_at: now,
        },
        entries,
    }
}
