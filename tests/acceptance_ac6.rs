//! AC6: The tool never writes to or deletes any scanned path.
//! A read-only fixture dir survives a full run unchanged.

use chrono::Utc;
use std::fs;
use tempfile::TempDir;

fn build_readonly_fixture() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    let proj = root.join("proj");
    fs::create_dir_all(proj.join("target")).expect("mkdir");
    fs::write(proj.join("Cargo.toml"), "[package]\nname=\"proj\"").expect("write");
    fs::write(proj.join("target/artifact"), b"important artifact").expect("write artifact");

    tmp
}

fn collect_snapshot(root: &std::path::Path) -> Vec<(std::path::PathBuf, u64)> {
    let mut entries = Vec::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.expect("walkdir entry");
        let meta = entry.metadata().expect("metadata");
        entries.push((entry.path().to_path_buf(), meta.len()));
    }
    entries.sort();
    entries
}

#[test]
fn test_read_only() {
    let tmp = build_readonly_fixture();
    let root = tmp.path().to_path_buf();
    let now = Utc::now();

    // Snapshot before.
    let before = collect_snapshot(&root);

    // Run the survey.
    let roots = &[root.clone()];
    let _output = ballast_survey::survey(roots, 0, now);

    // Snapshot after.
    let after = collect_snapshot(&root);

    assert_eq!(
        before, after,
        "filesystem was modified by survey — this is a bug"
    );
}
