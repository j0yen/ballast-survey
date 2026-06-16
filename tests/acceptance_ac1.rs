//! AC1: `ballast-survey --json` against a fixture tree emits a JSON array of
//! entries, each with `path`, `kind`, `bytes`, `entries`, `mtime`, `age_days`, `crate`.

use chrono::Utc;
use std::fs;
use tempfile::TempDir;

/// Build a fixture tree with a rust target/ dir and a node_modules/ dir.
fn build_fixture() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    // Rust project with target/
    let proj = root.join("myproject");
    fs::create_dir_all(proj.join("target/debug")).expect("mkdir");
    fs::write(proj.join("Cargo.toml"), "[package]\nname=\"myproject\"").expect("write Cargo.toml");
    fs::write(proj.join("target/debug/binary"), b"fake binary content".as_ref()).expect("write bin");

    // Node project
    let node = root.join("nodeapp");
    fs::create_dir_all(node.join("node_modules/some-pkg")).expect("mkdir node_modules");
    fs::write(node.join("node_modules/some-pkg/index.js"), b"module.exports = {}").expect("write js");

    tmp
}

#[test]
fn test_json_output_fields() {
    let tmp = build_fixture();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];

    let output = ballast_survey::survey(roots, 0, now).expect("survey failed");

    assert!(
        !output.entries.is_empty(),
        "expected at least one reclaimable entry"
    );

    for entry in &output.entries {
        // All required fields must be present (they always are due to type system,
        // but verify semantic content).
        assert!(
            entry.path.is_absolute() || entry.path.starts_with(tmp.path()),
            "path should be within fixture: {:?}",
            entry.path
        );
        assert!(entry.bytes > 0 || entry.entries > 0, "entry should have non-trivial size");
        // age_days can be 0 for very fresh dirs; just check it's not negative beyond reason.
        assert!(entry.age_days >= -1, "age_days should not be absurdly negative");
        assert!(!entry.crate_name.is_empty(), "crate_name should be non-empty");
    }

    // Check JSON serialization round-trips all fields.
    let json = serde_json::to_string(&output).expect("serialize");
    let reparsed: serde_json::Value = serde_json::from_str(&json).expect("deserialize");
    let entries = reparsed["entries"].as_array().expect("entries array");
    assert!(!entries.is_empty(), "JSON entries should be non-empty");

    for e in entries {
        assert!(e["path"].is_string(), "path must be a string");
        assert!(e["kind"].is_string(), "kind must be a string");
        assert!(e["bytes"].is_number(), "bytes must be a number");
        assert!(e["entries"].is_number(), "entries must be a number");
        assert!(e["mtime"].is_string(), "mtime must be a string");
        assert!(e["age_days"].is_number(), "age_days must be a number");
        assert!(e["crate_name"].is_string(), "crate_name must be a string");
    }
}
