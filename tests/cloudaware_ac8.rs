//! Cloud-aware AC8: Existing ballast-survey ACs still pass with the new
//! `cloud_info` field present; `cargo test` and `cargo clippy` are green.
//!
//! This test exercises the original `survey()` function (which now calls
//! `survey_with_options` with default options) and verifies backward
//! compatibility: existing callers see valid results without breaking.

use ballast_survey::survey;
use chrono::Utc;
use std::fs;
use tempfile::TempDir;

fn build_mixed_fixture() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    // Rust project
    let proj = root.join("rust_proj");
    fs::create_dir_all(proj.join("target/debug")).expect("mkdir rust target");
    fs::write(proj.join("Cargo.toml"), "[package]\nname=\"rust_proj\"").expect("write Cargo.toml");
    fs::write(proj.join("target/debug/binary"), b"bin").expect("write binary");

    // Node project
    let node = root.join("node_proj");
    fs::create_dir_all(node.join("node_modules/pkg")).expect("mkdir node");
    fs::write(node.join("node_modules/pkg/index.js"), b"x").expect("write js");

    tmp
}

#[test]
fn test_survey_still_returns_both_kinds() {
    let tmp = build_mixed_fixture();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];

    let output = survey(roots, 0, now);

    assert!(
        output.entries.len() >= 2,
        "should find at least 2 entries (rust + node)"
    );

    let has_rust = output
        .entries
        .iter()
        .any(|e| e.kind == ballast_survey::classify::EntryKind::RustTarget);
    let has_node = output
        .entries
        .iter()
        .any(|e| e.kind == ballast_survey::classify::EntryKind::NodeModules);

    assert!(has_rust, "should have a RustTarget entry");
    assert!(has_node, "should have a NodeModules entry");
}

#[test]
fn test_survey_json_round_trips_all_fields() {
    let tmp = build_mixed_fixture();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];
    let output = survey(roots, 0, now);

    let json = serde_json::to_string_pretty(&output).expect("serialize");
    let reparsed: serde_json::Value = serde_json::from_str(&json).expect("deserialize");
    let entries = reparsed["entries"].as_array().expect("entries array");

    assert!(!entries.is_empty(), "entries must be non-empty");

    for e in entries {
        assert!(e["path"].is_string(), "path must be string");
        assert!(e["kind"].is_string(), "kind must be string");
        assert!(e["bytes"].is_number(), "bytes must be number");
        assert!(e["entries"].is_number(), "entries must be number");
        assert!(e["mtime"].is_string(), "mtime must be string");
        assert!(e["age_days"].is_number(), "age_days must be number");
        assert!(e["crate_name"].is_string(), "crate_name must be string");
    }
}

#[test]
fn test_summary_reclaimable_bytes_equals_entry_sum() {
    let tmp = build_mixed_fixture();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];
    let output = survey(roots, 0, now);

    let expected: u64 = output.entries.iter().map(|e| e.bytes).sum();
    assert_eq!(
        output.summary.reclaimable_bytes,
        expected,
        "summary.reclaimable_bytes must equal sum of entry bytes"
    );
}
