//! Cloud-aware AC5: `install_source` records whether install state came from
//! `adopt` or a direct probe; with `adopt` absent the tool still classifies
//! via probe and exits 0.

use ballast_survey::cloudaware::{BinNameOverrides, InstallSource, classify_entry};
use chrono::{Duration, Utc};
use filetime::FileTime;
use std::fs;
use tempfile::TempDir;

fn build_crate_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join("target")).expect("mkdir");
    fs::write(tmp.path().join("Cargo.toml"), "[package]\nname=\"pkg\"").expect("write");
    tmp
}

#[test]
fn test_adopt_source_when_adopt_paths_provided() {
    let tmp = build_crate_dir();
    let bin_tmp = tempfile::tempdir().expect("bin tempdir");

    let bin_path = bin_tmp.path().join("pkg");
    fs::write(&bin_path, b"binary").expect("write binary");
    let bin_mtime = Utc::now() - Duration::hours(1);
    filetime::set_file_mtime(
        &bin_path,
        FileTime::from_unix_time(bin_mtime.timestamp(), 0),
    )
    .expect("set mtime");

    let target_mtime = Utc::now() - Duration::days(10);
    let adopt_paths = Some(vec![bin_path]);
    let overrides = BinNameOverrides::new();

    let info = classify_entry("pkg", tmp.path(), target_mtime, &overrides, &adopt_paths);

    // Binary was found via the adopt_paths list → source should be Adopt.
    assert_eq!(
        info.install_source,
        InstallSource::Adopt,
        "install_source should be Adopt when adopt_paths resolves the binary"
    );
}

#[test]
fn test_no_adopt_falls_back_to_probe() {
    let tmp = build_crate_dir();
    let target_mtime = Utc::now() - Duration::days(10);

    // Simulate `adopt` not available: pass None for adopt_paths.
    let adopt_paths = None;
    let overrides = BinNameOverrides::new();

    // This should NOT panic / error — it should fall back to probe and exit 0.
    let info = classify_entry("pkg", tmp.path(), target_mtime, &overrides, &adopt_paths);

    // Whatever the result, the install_source must be Probe when adopt is absent.
    assert_eq!(
        info.install_source,
        InstallSource::Probe,
        "install_source should be Probe when adopt is absent"
    );
    // Tool continued without error (we reach this line means exit 0).
}
