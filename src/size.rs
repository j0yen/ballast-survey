//! Directory size computation.
//!
//! Sums apparent file sizes (not block counts) and counts filesystem entries.

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
    /// Newest file mtime found within the subtree (UTC).
    pub mtime: DateTime<Utc>,
}

/// Recursively size a directory, returning total bytes, entry count, and newest file mtime.
///
/// Only regular file mtimes are tracked (not directories), because directory mtime
/// changes whenever entries are added/removed and does not represent the age of contents.
///
/// Follows symlinks only at the top level (not recursively) to avoid loops.
pub fn size_dir(path: &Path) -> SizeResult {
    let mut bytes: u64 = 0;
    let mut entries: u64 = 0;
    let mut newest: Option<std::time::SystemTime> = None;

    for entry in WalkDir::new(path).follow_links(false) {
        let Ok(entry) = entry else { continue };
        let Ok(meta) = entry.metadata() else { continue };

        entries += 1;

        if meta.is_file() {
            bytes += meta.len();
            // Only track file mtimes.
            if let Ok(mtime) = meta.modified() {
                newest = Some(newest.map_or(mtime, |prev: std::time::SystemTime| prev.max(mtime)));
            }
        }
    }

    let mtime = newest.map_or_else(
        || Utc.timestamp_opt(0, 0).single().unwrap_or(DateTime::<Utc>::MIN_UTC),
        system_time_to_utc,
    );

    SizeResult {
        bytes,
        entries,
        mtime,
    }
}

fn system_time_to_utc(st: std::time::SystemTime) -> DateTime<Utc> {
    let duration = st
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Safe: timestamps from the OS are within i64 range for the foreseeable future.
    // The duration is since Unix epoch; u64 seconds won't exceed i64::MAX until year 292B.
    let secs = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
    Utc.timestamp_opt(secs, duration.subsec_nanos())
        .single()
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}
