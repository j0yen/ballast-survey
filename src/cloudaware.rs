//! Cloud-aware fossil detection for Rust `target/` directories.
//!
//! Extends each `RustTarget` entry with:
//! - `installed_bin` — resolved path of the crate's installed binary (if any)
//! - `installed_mtime` — mtime of the installed binary
//! - `installed_newer_than_target` — bool: binary newer than target mtime
//! - `cloud_built` — bool: crate is known to route builds to the cloud
//! - `fossil` — bool: `installed_newer_than_target && installed_bin.is_some()`
//! - `reap_safety` — ranked enum for sort/prioritisation
//! - `install_source` — `adopt` or `probe`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// How `reap_safety` was determined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallSource {
    /// Install state determined by shelling out to `adopt`.
    Adopt,
    /// Install state determined by a direct `~/.local/bin` filesystem probe.
    Probe,
}

/// Ranked safety level for reclaiming a `target/` directory.
///
/// Variants are ordered from highest to lowest reclaim confidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReapSafety {
    /// Installed binary is **newer** than the local `target/` — near-zero risk.
    Fossil,
    /// Installed binary exists but is **older** than the target (may be stale).
    StaleInstalled,
    /// No installed binary found — target may be the only copy of the built artifact.
    StaleUninstalled,
    /// Target is recent (younger than a practical stale threshold).
    Recent,
}

impl ReapSafety {
    /// Numeric rank — lower is safer to reclaim (higher priority reap).
    #[must_use]
    pub fn rank(&self) -> u8 {
        match self {
            Self::Fossil => 0,
            Self::StaleInstalled => 1,
            Self::StaleUninstalled => 2,
            Self::Recent => 3,
        }
    }
}

/// Cloud-aware extension fields added to a `RustTarget` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAwareInfo {
    /// Resolved path of the installed binary, if found.
    pub installed_bin: Option<PathBuf>,
    /// Mtime of the installed binary (UTC), if found.
    pub installed_mtime: Option<DateTime<Utc>>,
    /// True when the installed binary's mtime is strictly newer than the target mtime.
    pub installed_newer_than_target: bool,
    /// True if the crate is detected as routing builds to the cloud.
    pub cloud_built: bool,
    /// True when `installed_newer_than_target && installed_bin.is_some()`.
    ///
    /// `cloud_built` strengthens confidence but is not required — the mtime
    /// proof is sufficient on its own.
    pub fossil: bool,
    /// Ranked reap-safety label.
    pub reap_safety: ReapSafety,
    /// Whether install state was resolved via `adopt` or a direct probe.
    pub install_source: InstallSource,
}

/// Override map: crate name → binary name (for crates where binary ≠ crate name).
///
/// Example: `{"wintermute-brain": "wmd"}`.
pub type BinNameOverrides = HashMap<String, String>;

/// Check whether a crate's builds route to the cloud by inspecting:
///
/// 1. A `.autobuilder_cloud` marker file in the crate directory.
/// 2. A `cloud:` field in an `autobuilder.json` / `autobuilder.yaml` manifest in
///    the crate directory.
/// 3. The global `AUTOBUILDER_CLOUD` environment variable being set to `1` or `true`.
///
/// Detection stops at the first positive hit; the precedence order is as listed.
fn detect_cloud_built(crate_dir: &Path) -> bool {
    // 1. Marker file
    if crate_dir.join(".autobuilder_cloud").exists() {
        return true;
    }

    // 2. autobuilder manifest field `cloud: true`
    let manifest_candidates = [
        crate_dir.join("autobuilder.json"),
        crate_dir.join(".autobuilder.json"),
    ];
    for manifest in &manifest_candidates {
        if let Ok(content) = std::fs::read_to_string(manifest) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if v.get("cloud").and_then(|c| c.as_bool()).unwrap_or(false) {
                    return true;
                }
            }
        }
    }

    // 3. Environment variable
    matches!(
        std::env::var("AUTOBUILDER_CLOUD").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Run `adopt --list` (if available) and parse JSON output to discover installed
/// paths.  Returns `None` if `adopt` is not on `$PATH` or exits non-zero.
///
/// Expected output format: a JSON array of objects with at least a `"path"` key
/// (the installed binary path) and optionally a `"name"` key.  We collect all
/// paths into a `Vec<PathBuf>` for the caller to search.
fn run_adopt() -> Option<Vec<PathBuf>> {
    let output = std::process::Command::new("adopt")
        .arg("--list")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let val: serde_json::Value = serde_json::from_str(&text).ok()?;
    let arr = val.as_array()?;
    let paths: Vec<PathBuf> = arr
        .iter()
        .filter_map(|item| item.get("path")?.as_str())
        .map(PathBuf::from)
        .collect();
    if paths.is_empty() { None } else { Some(paths) }
}

/// Probe `~/.local/bin` directly for a binary named `bin_name`.
///
/// Returns the resolved path and its mtime if found.
fn probe_local_bin(bin_name: &str) -> Option<(PathBuf, DateTime<Utc>)> {
    let home = std::env::var("HOME").ok()?;
    let candidate = PathBuf::from(home).join(".local/bin").join(bin_name);
    if candidate.exists() {
        let meta = std::fs::metadata(&candidate).ok()?;
        let mtime = file_mtime(&meta)?;
        Some((candidate, mtime))
    } else {
        None
    }
}

/// Extract mtime from filesystem metadata as UTC.
fn file_mtime(meta: &std::fs::Metadata) -> Option<DateTime<Utc>> {
    use std::time::SystemTime;
    let st = meta.modified().ok()?;
    let duration = st.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    #[allow(clippy::cast_possible_truncation)]
    let secs = duration.as_secs() as i64;
    DateTime::from_timestamp(secs, 0)
}

/// Build the `CloudAwareInfo` for a single `RustTarget` entry.
///
/// - `crate_name` is the name of the owning crate directory (the parent of `target/`).
/// - `target_mtime` is the most-recently-modified file mtime inside the `target/` tree.
/// - `overrides` maps crate names to binary names.
/// - `adopt_paths` is the pre-computed list of paths from `adopt --list` (or `None`).
#[must_use]
pub fn classify_entry(
    crate_name: &str,
    crate_dir: &Path,
    target_mtime: DateTime<Utc>,
    overrides: &BinNameOverrides,
    adopt_paths: &Option<Vec<PathBuf>>,
) -> CloudAwareInfo {
    // Determine the binary name to look for.
    let bin_name = overrides
        .get(crate_name)
        .map(String::as_str)
        .unwrap_or(crate_name);

    // 1. Try `adopt` output first.
    let mut install_source = InstallSource::Probe;
    let mut installed_bin: Option<PathBuf> = None;
    let mut installed_mtime: Option<DateTime<Utc>> = None;

    if let Some(paths) = adopt_paths {
        // Search the adopt list for a binary matching our bin_name.
        let matched = paths.iter().find(|p| {
            p.file_name()
                .map(|f| f.to_string_lossy() == bin_name)
                .unwrap_or(false)
        });
        if let Some(path) = matched {
            if let Ok(meta) = std::fs::metadata(path) {
                installed_bin = Some(path.clone());
                installed_mtime = file_mtime(&meta);
                install_source = InstallSource::Adopt;
            }
        }
    }

    // 2. Fall back to direct `~/.local/bin` probe if adopt didn't resolve.
    if installed_bin.is_none() {
        if let Some((path, mtime)) = probe_local_bin(bin_name) {
            installed_bin = Some(path);
            installed_mtime = Some(mtime);
            install_source = InstallSource::Probe;
        }
    }

    // Compute derived booleans.
    let installed_newer_than_target = match installed_mtime {
        Some(imt) => imt > target_mtime,
        None => false,
    };

    let cloud_built = detect_cloud_built(crate_dir);
    let fossil = installed_bin.is_some() && installed_newer_than_target;

    let reap_safety = if fossil {
        ReapSafety::Fossil
    } else if installed_bin.is_some() {
        ReapSafety::StaleInstalled
    } else {
        ReapSafety::StaleUninstalled
    };

    CloudAwareInfo {
        installed_bin,
        installed_mtime,
        installed_newer_than_target,
        cloud_built,
        fossil,
        reap_safety,
        install_source,
    }
}

/// Annotate all `RustTarget` entries in `entries` with cloud-aware information.
///
/// Non-`RustTarget` entries are left unchanged (their `cloud_info` field is `None`).
///
/// `overrides` maps crate names to binary names for crates whose installed binary
/// does not match the crate directory name.
pub fn annotate_entries(
    entries: &mut Vec<crate::classify::SurveyEntry>,
    overrides: &BinNameOverrides,
) {
    use crate::classify::EntryKind;

    // Try `adopt` once for the whole batch.
    let adopt_paths = run_adopt();

    for entry in entries.iter_mut() {
        if entry.kind != EntryKind::RustTarget {
            continue;
        }
        // The crate directory is the parent of the `target/` directory.
        let crate_dir = entry
            .path
            .parent()
            .unwrap_or(entry.path.as_path());

        entry.cloud_info = Some(classify_entry(
            &entry.crate_name,
            crate_dir,
            entry.mtime,
            overrides,
            &adopt_paths,
        ));
    }
}
