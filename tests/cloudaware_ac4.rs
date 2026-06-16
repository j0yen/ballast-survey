//! Cloud-aware AC4: `cloud_built` is set from the documented detection
//! precedence and recorded; its absence does not clear `fossil` — the mtime
//! proof stands alone.

use ballast_survey::cloudaware::{BinNameOverrides, classify_entry};
use chrono::{Duration, Utc};
use filetime::FileTime;
use std::fs;
use tempfile::TempDir;

fn build_crate_with_newer_bin(cloud_marker: bool) -> (TempDir, TempDir) {
    let proj_tmp = tempfile::tempdir().expect("proj tempdir");
    let bin_tmp = tempfile::tempdir().expect("bin tempdir");

    fs::create_dir_all(proj_tmp.path().join("target")).expect("mkdir");
    fs::write(proj_tmp.path().join("Cargo.toml"), "[package]\nname=\"mypkg\"").expect("write");

    if cloud_marker {
        fs::write(proj_tmp.path().join(".autobuilder_cloud"), b"").expect("write marker");
    }

    let bin_path = bin_tmp.path().join("mypkg");
    fs::write(&bin_path, b"bin").expect("write bin");
    let bin_mtime = Utc::now() - Duration::hours(2);
    filetime::set_file_mtime(
        &bin_path,
        FileTime::from_unix_time(bin_mtime.timestamp(), 0),
    )
    .expect("set mtime");

    (proj_tmp, bin_tmp)
}

#[test]
fn test_fossil_true_without_cloud_marker() {
    // No cloud marker file, binary is newer → fossil must still be true even without cloud_built.
    let (proj_tmp, bin_tmp) = build_crate_with_newer_bin(false);
    let target_mtime = Utc::now() - Duration::days(15);

    let bin_path = bin_tmp.path().join("mypkg");
    let adopt_paths = Some(vec![bin_path]);
    let overrides = BinNameOverrides::new();

    let info = classify_entry("mypkg", proj_tmp.path(), target_mtime, &overrides, &adopt_paths);

    // cloud_built may be true if AUTOBUILDER_CLOUD is set in env, so we
    // only check the relationship: fossil is driven by mtime proof, not cloud_built.
    assert!(
        info.fossil,
        "fossil must be true from mtime proof alone (cloud_built is {:?} but not required)",
        info.cloud_built
    );
    // In either case, fossil AND cloud_built may both be true or just fossil.
    // The invariant: installed_newer_than_target => fossil (regardless of cloud_built).
    assert!(
        info.installed_newer_than_target,
        "installed_newer_than_target must be true"
    );
}

#[test]
fn test_cloud_built_set_from_marker_file() {
    let (proj_tmp, bin_tmp) = build_crate_with_newer_bin(true);
    let target_mtime = Utc::now() - Duration::days(15);

    let bin_path = bin_tmp.path().join("mypkg");
    let adopt_paths = Some(vec![bin_path]);
    let overrides = BinNameOverrides::new();

    let info = classify_entry("mypkg", proj_tmp.path(), target_mtime, &overrides, &adopt_paths);

    assert!(info.cloud_built, "cloud_built should be true from .autobuilder_cloud marker");
    assert!(info.fossil, "fossil must be true when binary is newer");
}

#[test]
fn test_cloud_built_set_from_autobuilder_json() {
    let proj_tmp = tempfile::tempdir().expect("proj tempdir");
    let bin_tmp = tempfile::tempdir().expect("bin tempdir");

    fs::create_dir_all(proj_tmp.path().join("target")).expect("mkdir");
    fs::write(proj_tmp.path().join("Cargo.toml"), "[package]\nname=\"jsonpkg\"").expect("write");

    // Write autobuilder.json with cloud: true
    fs::write(
        proj_tmp.path().join("autobuilder.json"),
        b"{\"cloud\": true}",
    )
    .expect("write autobuilder.json");

    let bin_path = bin_tmp.path().join("jsonpkg");
    fs::write(&bin_path, b"bin").expect("write bin");
    let bin_mtime = Utc::now() - Duration::hours(1);
    filetime::set_file_mtime(
        &bin_path,
        FileTime::from_unix_time(bin_mtime.timestamp(), 0),
    )
    .expect("set mtime");

    let target_mtime = Utc::now() - Duration::days(10);
    let adopt_paths = Some(vec![bin_path]);
    let overrides = BinNameOverrides::new();

    let info = classify_entry("jsonpkg", proj_tmp.path(), target_mtime, &overrides, &adopt_paths);

    assert!(
        info.cloud_built,
        "cloud_built should be true from autobuilder.json with cloud: true"
    );
}
