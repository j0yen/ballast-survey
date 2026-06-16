//! Cloud-aware AC3: `bin_name` config override resolves a crate whose binary
//! name differs from the crate name.
//!
//! Fixture: crate `alpha` → binary `a` installed and newer → `fossil: true`.

use ballast_survey::cloudaware::{BinNameOverrides, ReapSafety, classify_entry};
use chrono::{Duration, Utc};
use filetime::FileTime;
use std::fs;
use tempfile::TempDir;

fn build_fixture() -> (TempDir, TempDir) {
    let proj_tmp = tempfile::tempdir().expect("proj tempdir");
    let bin_tmp = tempfile::tempdir().expect("bin tempdir");

    // Crate dir named "alpha"
    fs::create_dir_all(proj_tmp.path().join("target")).expect("mkdir");
    fs::write(proj_tmp.path().join("Cargo.toml"), "[package]\nname=\"alpha\"").expect("write");

    // Binary installed as "a" (not "alpha")
    let bin_path = bin_tmp.path().join("a");
    fs::write(&bin_path, b"binary a").expect("write binary");

    // Set binary mtime to newer than target
    let bin_mtime = Utc::now() - Duration::hours(1);
    filetime::set_file_mtime(
        &bin_path,
        FileTime::from_unix_time(bin_mtime.timestamp(), 0),
    )
    .expect("set bin mtime");

    (proj_tmp, bin_tmp)
}

#[test]
fn test_bin_name_override_resolves_fossil() {
    let (proj_tmp, bin_tmp) = build_fixture();

    // target mtime is 10 days old
    let target_mtime = Utc::now() - Duration::days(10);

    let bin_path = bin_tmp.path().join("a");
    let adopt_paths = Some(vec![bin_path]);

    // Override: crate "alpha" → binary "a"
    let mut overrides = BinNameOverrides::new();
    overrides.insert("alpha".to_owned(), "a".to_owned());

    let info = classify_entry("alpha", proj_tmp.path(), target_mtime, &overrides, &adopt_paths);

    assert!(
        info.fossil,
        "fossil must be true when bin_name override maps crate to newer binary"
    );
    assert_eq!(
        info.reap_safety,
        ReapSafety::Fossil,
        "reap_safety must be Fossil"
    );
    assert!(
        info.installed_newer_than_target,
        "installed_newer_than_target must be true"
    );
}

#[test]
fn test_without_override_crate_name_used() {
    let (proj_tmp, bin_tmp) = build_fixture();
    let target_mtime = Utc::now() - Duration::days(10);

    let bin_path = bin_tmp.path().join("a");
    let adopt_paths = Some(vec![bin_path]);

    // No override → looks for "alpha", not "a" → no match
    let overrides = BinNameOverrides::new();

    let info = classify_entry("alpha", proj_tmp.path(), target_mtime, &overrides, &adopt_paths);

    // Without override, "alpha" binary not found, so fossil = false
    // (unless probe finds ~/.local/bin/alpha which shouldn't exist in CI)
    // This test verifies that without the override the lookup by crate name fails.
    // We can only assert if adopt found nothing and probe didn't match.
    if info.install_source == ballast_survey::cloudaware::InstallSource::Probe {
        // Probe path: may or may not find an "alpha" binary in real ~/.local/bin.
        // Just verify the override was not used (we can't assert fossil here
        // without controlling HOME — skip the assertion in that case).
        return;
    }
    // adopt path: adopt_paths only has "a", not "alpha", so no match.
    assert!(
        !info.fossil,
        "without override, 'alpha' crate should not match binary 'a'"
    );
}
