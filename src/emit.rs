//! Output types for the survey result.

use crate::classify::SurveyEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Summary header for the survey output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    /// Total reclaimable bytes across all entries.
    pub reclaimable_bytes: u64,
    /// Number of entries after filtering.
    pub entry_count: usize,
    /// The `now` instant supplied by the caller (used for age computation).
    pub scanned_at: DateTime<Utc>,
}

/// Full survey output: summary header + sorted entry list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    /// Aggregate summary.
    pub summary: Summary,
    /// Entries sorted by `bytes` descending, filtered by `min_bytes`.
    pub entries: Vec<SurveyEntry>,
}
