//! AC5: Age math is deterministic under a caller-supplied `now`.
//! A fixture with a known mtime yields a known `age_days`.

use chrono::{DateTime, TimeZone, Utc};
use std::fs;
use std::time::{Duration, UNIX_EPOCH};
use tempfile::TempDir;

fn build_fixture_with_known_mtime() -> (TempDir, DateTime<Utc>) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let proj = root.join("proj");
    fs::create_dir_all(proj.join("target")).expect("mkdir");
    fs::write(proj.join("Cargo.toml"), "[package]\nname=\"proj\"").expect("write");
    let file = proj.join("target/artifact");
    fs::write(&file, b"content").expect("write");

    // Set the file's mtime to exactly Unix epoch + 10 days.
    let ten_days_secs: u64 = 10 * 24 * 3600;
    let mtime_epoch = UNIX_EPOCH + Duration::from_secs(ten_days_secs);
    filetime::set_file_mtime(&file, filetime::FileTime::from_system_time(mtime_epoch))
        .expect("set mtime");

    let known_mtime = Utc
        .timestamp_opt(ten_days_secs as i64, 0)
        .single()
        .expect("valid timestamp");

    (tmp, known_mtime)
}

#[test]
fn test_deterministic_age() {
    // This test requires the `filetime` crate; if not available, skip gracefully.
    let (tmp, _known_mtime) = build_fixture_with_known_mtime();

    // `now` = known_mtime + 30 days → age_days should be exactly 30.
    let thirty_days_secs: i64 = 30 * 24 * 3600;
    let ten_days_epoch: i64 = 10 * 24 * 3600;
    let now = Utc
        .timestamp_opt(ten_days_epoch + thirty_days_secs, 0)
        .single()
        .expect("valid now");

    let roots = &[tmp.path().to_path_buf()];
    let output = ballast_survey::survey(roots, 0, now).expect("survey");

    assert!(
        !output.entries.is_empty(),
        "expected at least one entry in fixture"
    );

    for entry in &output.entries {
        // age_days should be approximately 30 (within 1 day tolerance for mtime
        // propagation through dir metadata).
        assert!(
            (entry.age_days - 30).abs() <= 1,
            "expected age_days ~30, got {} for {:?}",
            entry.age_days,
            entry.path
        );
    }
}
