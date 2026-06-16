//! Cloud-aware AC6: `--no-cloudaware` reproduces the v0.1 survey output
//! exactly (no new fields).

use ballast_survey::{SurveyOptions, survey_with_options};
use chrono::Utc;
use std::fs;
use tempfile::TempDir;

fn build_fixture() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let proj = root.join("crate_a");
    fs::create_dir_all(proj.join("target/debug")).expect("mkdir target");
    fs::write(proj.join("Cargo.toml"), "[package]\nname=\"crate_a\"").expect("write Cargo.toml");
    fs::write(proj.join("target/debug/out"), b"content").expect("write artifact");

    tmp
}

#[test]
fn test_no_cloudaware_omits_cloud_info() {
    let tmp = build_fixture();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];

    let opts = SurveyOptions {
        no_cloudaware: true,
        ..Default::default()
    };

    let output = survey_with_options(roots, 0, now, &opts);

    assert!(!output.entries.is_empty(), "should have at least one entry");
    for e in &output.entries {
        assert!(
            e.cloud_info.is_none(),
            "cloud_info must be None when --no-cloudaware is set, got: {:?}",
            e.cloud_info
        );
    }
}

#[test]
fn test_cloudaware_enabled_populates_cloud_info_for_rust_targets() {
    use ballast_survey::classify::EntryKind;

    let tmp = build_fixture();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];

    let opts = SurveyOptions::default(); // cloud-aware ON

    let output = survey_with_options(roots, 0, now, &opts);

    let rust_entries: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::RustTarget)
        .collect();

    assert!(!rust_entries.is_empty(), "should have at least one RustTarget entry");
    for e in rust_entries {
        assert!(
            e.cloud_info.is_some(),
            "RustTarget entry should have cloud_info when cloud-aware is enabled: {:?}",
            e.path
        );
    }
}

#[test]
fn test_no_cloudaware_json_round_trips_without_cloud_fields() {
    let tmp = build_fixture();
    let now = Utc::now();
    let roots = &[tmp.path().to_path_buf()];

    let opts = SurveyOptions {
        no_cloudaware: true,
        ..Default::default()
    };
    let output = survey_with_options(roots, 0, now, &opts);

    let json = serde_json::to_string(&output).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("deserialize");
    let entries = parsed["entries"].as_array().expect("entries array");

    for e in entries {
        assert!(
            e.get("cloud_info").is_none() || e["cloud_info"].is_null(),
            "cloud_info must not appear in --no-cloudaware JSON output"
        );
    }
}
