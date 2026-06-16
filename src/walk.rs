//! Directory walker that finds reclaimable subtrees.
//!
//! Uses a manual stack-based walk with early pruning: once a reclaimable dir
//! is matched, it is not descended into (preventing double-counting of nested
//! reclaimable dirs, e.g. `node_modules` inside `target/`).

use crate::classify::{EntryKind, SurveyEntry};
use crate::size::size_dir;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};

/// Scan a single root directory for reclaimable subtrees.
///
/// Returns all matched entries with their sizes and metadata.  The walk does
/// **not** descend into matched directories, so nested reclaimable dirs are
/// counted as part of their outermost ancestor.
///
/// # Errors
/// Returns an error if `root` cannot be read.
pub fn scan_root(root: &Path, now: DateTime<Utc>) -> Result<Vec<SurveyEntry>> {
    let mut results: Vec<SurveyEntry> = Vec::new();
    // BFS queue of directories to process.
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root.to_path_buf());

    while let Some(dir) = queue.pop_front() {
        // Read directory entries; skip unreadable dirs silently.
        let read_dir = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for entry_result in read_dir {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if !ft.is_dir() {
                continue;
            }

            let child_path = entry.path();
            let child_name = entry.file_name();
            let name = child_name.to_string_lossy();

            if let Some(kind) = classify_dir_name(name.as_ref(), &child_path) {
                let size = match size_dir(&child_path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let age_days = (now - size.mtime).num_days();
                let crate_name = parent_name(&child_path);

                results.push(SurveyEntry {
                    path: child_path,
                    kind,
                    bytes: size.bytes,
                    entries: size.entries,
                    mtime: size.mtime,
                    age_days,
                    crate_name,
                });
                // Do NOT enqueue this dir — don't descend into matched dirs.
            } else {
                // Not reclaimable at this level — enqueue for further descent.
                queue.push_back(child_path);
            }
        }
    }

    Ok(results)
}

/// Classify a directory by its name and path, returning the kind if it is
/// reclaimable, or `None` if it should be descended into.
fn classify_dir_name(name: &str, path: &Path) -> Option<EntryKind> {
    match name {
        "node_modules" => Some(EntryKind::NodeModules),
        ".venv" => Some(EntryKind::Venv),
        "__pycache__" => Some(EntryKind::Pycache),
        "target" => {
            if is_rust_target(path) {
                Some(EntryKind::RustTarget)
            } else {
                None
            }
        }
        _ => {
            if is_cargo_cache(path) {
                Some(EntryKind::CargoCache)
            } else {
                None
            }
        }
    }
}

/// Returns true if `path` looks like a Rust build `target/` directory.
///
/// Heuristic: parent directory contains `Cargo.toml`, OR the target dir itself
/// contains `.rustc_info.json` or `CARGO_OK`.
fn is_rust_target(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        if parent.join("Cargo.toml").exists() {
            return true;
        }
    }
    path.join(".rustc_info.json").exists() || path.join("CARGO_OK").exists()
}

/// Returns true if this directory is a cargo registry cache or git checkouts dir.
fn is_cargo_cache(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    (path_str.contains("/.cargo/registry/cache") || path_str.contains("/.cargo/git/checkouts"))
        && path.is_dir()
}

/// Returns the immediate parent directory's name as the "crate" or owning dir.
fn parent_name(path: &Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<root>".to_owned())
}
