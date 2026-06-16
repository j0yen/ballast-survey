//! Directory size computation.
//!
//! Sums apparent file sizes (not block counts) and counts filesystem entries.

use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use std::path::Path;
use walkdir::WalkDir;

/// Result of sizing a single directory subtree.
#[derive(Debug, Clone)]
pub struct SizeResult {
    /// Total apparent size in bytes.
    pub bytes: u64,
    /// Total number of filesystem entries (files + dirs).
    pub entries: u64,
    /// Newest mtime found within the subtree (UTC).
    pub mtime: DateTime<Utc>,
}

/// Recursively size a directory, returning total bytes, entry count, and newest mtime.
///
/// Follows symlinks only at the top level (not recursively) to avoid loops.
///
/// # Errors
/// Returns an error if the path cannot be walked.
pub fn size_dir(path: &Path) -> Result<SizeResult> {
    let mut bytes: u64 = 0;
    let mut entries: u64 = 0;
    let mut newest: Option<std::time::SystemTime> = None;

    for entry in WalkDir::new(path).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            // Skip unreadable entries (permissions) without failing the whole scan.
            Err(_) => continue,
        };

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        entries += 1;

        if meta.is_file() {
            bytes += meta.len();
        }

        if let Ok(mtime) = meta.modified() {
            newest = Some(match newest {
                Some(prev) => prev.max(mtime),
                None => mtime,
            });
        }
    }

    let mtime = newest
        .map(system_time_to_utc)
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().unwrap_or(DateTime::<Utc>::MIN_UTC));

    Ok(SizeResult {
        bytes,
        entries,
        mtime,
    })
}

fn system_time_to_utc(st: std::time::SystemTime) -> DateTime<Utc> {
    let duration = st
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    Utc.timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos())
        .single()
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}
