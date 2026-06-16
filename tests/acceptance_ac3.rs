//! AC3: A nested reclaimable dir (e.g. a `node_modules` inside a `target`) is
//! counted exactly once — the outer match wins, no double-counting.

use chrono::Utc;
use std::fs;
use tempfile::TempDir;

fn build_nested_fixture() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    // target/ is the outer match.
    let proj = root.join("proj");
    fs::create_dir_all(proj.join("target")).expect("mkdir target");
    fs::write(proj.join("Cargo.toml"), "[package]\nname=\"proj\"").expect("write");
    fs::write(proj.join("target/file.txt"), b"contents").expect("write file");

    // node_modules inside target/ — should NOT be a separate entry.
    let nested = proj.join("target/node_modules/pkg");
    fs::create_dir_all(&nested).expect("mkdir nested");
    fs::write(nested.join("index.js"), b"// nested").expect("write nested");

    tmp
}

#[test]
fn test_no_double_counting() {
    let tmp = build_nested_fixture();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];

    let output = ballast_survey::survey(roots, 0, now).expect("survey");

    // The target/ dir should be found exactly once.
    let target_entries: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.path.file_name().map(|n| n == "target").unwrap_or(false))
        .collect();

    assert_eq!(
        target_entries.len(),
        1,
        "expected exactly 1 target/ entry, got {}: {:?}",
        target_entries.len(),
        target_entries.iter().map(|e| &e.path).collect::<Vec<_>>()
    );

    // The nested node_modules should NOT appear as a separate entry.
    let node_entries: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.path.file_name().map(|n| n == "node_modules").unwrap_or(false))
        .collect();

    assert_eq!(
        node_entries.len(),
        0,
        "nested node_modules should not be a separate entry, found: {:?}",
        node_entries.iter().map(|e| &e.path).collect::<Vec<_>>()
    );
}
