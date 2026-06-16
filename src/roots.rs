//! Root resolution — expand `~` and validate that roots are readable directories.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Expand a single root path, resolving `~` to the home directory.
///
/// # Errors
/// Returns an error if:
/// - The path begins with `~` but no home directory can be found.
/// - The resolved path is not a readable directory.
pub fn expand_root(raw: &str) -> Result<PathBuf> {
    let expanded = if let Some(rest) = raw.strip_prefix("~/") {
        let home = home_dir().context("cannot resolve ~: no home directory found")?;
        home.join(rest)
    } else if raw == "~" {
        home_dir().context("cannot resolve ~: no home directory found")?
    } else {
        PathBuf::from(raw)
    };

    // Validate readability
    std::fs::read_dir(&expanded)
        .with_context(|| format!("cannot read root directory: {}", expanded.display()))?;

    Ok(expanded)
}

/// Expand a slice of raw root strings.
///
/// # Errors
/// Returns the first error encountered.
pub fn expand_roots(raws: &[String]) -> Result<Vec<PathBuf>> {
    raws.iter().map(|r| expand_root(r)).collect()
}

fn home_dir() -> Option<PathBuf> {
    // std::env::home_dir() is deprecated on Windows but fine on Linux.
    // We target Linux only (per PRD non-goals).
    #[allow(deprecated)]
    std::env::home_dir()
}
