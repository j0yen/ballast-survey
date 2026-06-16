//! Cloud-aware AC2: A crate with no installed binary is `fossil: false`,
//! `reap_safety: "stale-uninstalled"` (the protect-this case).

use ballast_survey::cloudaware::{BinNameOverrides, ReapSafety, classify_entry};
use chrono::{Duration, Utc};
use tempfile::TempDir;
use std::fs;

fn build_crate_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("target")).expect("mkdir");
    fs::write(tmp.path().join("Cargo.toml"), "[package]\nname=\"orphan\"").expect("write");
    tmp
}

#[test]
fn test_no_installed_binary_is_stale_uninstalled() {
    let tmp = build_crate_dir();
    let target_mtime = Utc::now() - Duration::days(30);

    // Supply empty adopt list (no binary) and ensure probe can't find anything
    // (binary name "orphan" won't exist in a test HOME).
    let adopt_paths = Some(vec![]);
    let overrides = BinNameOverrides::new();

    let info = classify_entry("orphan", tmp.path(), target_mtime, &overrides, &adopt_paths);

    assert!(!info.fossil, "fossil must be false when no binary installed");
    assert!(!info.installed_newer_than_target, "installed_newer must be false");
    assert!(info.installed_bin.is_none(), "installed_bin must be None");
    assert_eq!(
        info.reap_safety,
        ReapSafety::StaleUninstalled,
        "reap_safety must be stale-uninstalled when no binary exists"
    );
}
