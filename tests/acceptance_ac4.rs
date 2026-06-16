//! AC4: `--min-size 100M` excludes every entry below the floor.
//! Tested via the library's `min_bytes` parameter directly.

use chrono::Utc;
use std::fs;
use tempfile::TempDir;

fn build_fixture_small() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let proj = root.join("proj");
    fs::create_dir_all(proj.join("target")).expect("mkdir");
    fs::write(proj.join("Cargo.toml"), "[package]\nname=\"proj\"").expect("write");
    // Write only 1KB — well below 100M.
    fs::write(proj.join("target/tiny"), vec![b'y'; 1024]).expect("write tiny");

    tmp
}

#[test]
fn test_min_size_filter() {
    let tmp = build_fixture_small();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];

    // Without floor: should find the small entry.
    let without_floor = ballast_survey::survey(roots, 0, now).expect("survey");
    assert!(
        !without_floor.entries.is_empty(),
        "expected entries without min-size filter"
    );

    // With 100M floor (104_857_600 bytes): tiny entry should be excluded.
    let min_100m: u64 = 100 * 1024 * 1024;
    let with_floor = ballast_survey::survey(roots, min_100m, now).expect("survey");
    assert!(
        with_floor.entries.is_empty(),
        "expected no entries above 100M floor for tiny fixture, got: {:?}",
        with_floor.entries.iter().map(|e| (e.bytes, &e.path)).collect::<Vec<_>>()
    );
}
