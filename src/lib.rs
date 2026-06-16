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
pub mod emit;
pub mod roots;
pub mod size;
pub mod walk;

pub use classify::{EntryKind, SurveyEntry};
pub use emit::{Output, Summary};
pub use walk::scan_root;

use chrono::{DateTime, Utc};
use std::path::Path;

/// Run a full survey over `roots`, filtering by `min_bytes`.
///
/// `now` is the reference instant used for `age_days` computation — pass the
/// wall-clock value at the CLI boundary, or a fixed instant in tests.
#[must_use]
pub fn survey(
    roots: &[impl AsRef<Path>],
    min_bytes: u64,
    now: DateTime<Utc>,
) -> Output {
    let mut entries: Vec<SurveyEntry> = Vec::new();

    for root in roots {
        let root = root.as_ref();
        let mut found = walk::scan_root(root, now);
        entries.append(&mut found);
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
