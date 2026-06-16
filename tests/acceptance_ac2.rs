//! AC2: Entries are sorted by `bytes` descending; the summary header reports
//! `reclaimable_bytes` equal to the sum of entry `bytes`.

use chrono::Utc;
use std::fs;
use tempfile::TempDir;

fn build_fixture_two_sizes() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    // Large rust target
    let proj_a = root.join("proj_a");
    fs::create_dir_all(proj_a.join("target")).expect("mkdir");
    fs::write(proj_a.join("Cargo.toml"), "[package]\nname=\"proj_a\"").expect("write");
    // Write ~2KB of content
    fs::write(proj_a.join("target/large_file"), vec![b'x'; 2048]).expect("write large");

    // Smaller node_modules
    let proj_b = root.join("proj_b");
    fs::create_dir_all(proj_b.join("node_modules")).expect("mkdir");
    fs::write(proj_b.join("node_modules/tiny.js"), b"x").expect("write tiny");

    tmp
}

#[test]
fn test_sorted_and_summary() {
    let tmp = build_fixture_two_sizes();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];

    let output = ballast_survey::survey(roots, 0, now);

    // Entries must be sorted descending by bytes.
    let entries = &output.entries;
    assert!(entries.len() >= 2, "expected at least 2 entries");

    for window in entries.windows(2) {
        assert!(
            window[0].bytes >= window[1].bytes,
            "entries not sorted: {} < {}",
            window[0].bytes,
            window[1].bytes,
        );
    }

    // Summary reclaimable_bytes == sum of entry bytes.
    let expected_sum: u64 = entries.iter().map(|e| e.bytes).sum();
    assert_eq!(
        output.summary.reclaimable_bytes,
        expected_sum,
        "summary.reclaimable_bytes should equal sum of entry bytes"
    );
}
