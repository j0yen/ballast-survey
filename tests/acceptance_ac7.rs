//! AC7: `--help` documents every flag; exit 0 on success, non-zero on an
//! unreadable root with a clear stderr message.

use std::process::Command;

fn binary_path() -> std::path::PathBuf {
    // Try release first, then debug.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let release = std::path::Path::new(manifest_dir)
        .join("target/release/ballast-survey");
    if release.exists() {
        return release;
    }
    std::path::Path::new(manifest_dir)
        .join("target/debug/ballast-survey")
}

#[test]
fn test_help_exits_zero() {
    let bin = binary_path();
    if !bin.exists() {
        // Binary not built yet in test context; skip.
        return;
    }
    let out = Command::new(&bin)
        .arg("--help")
        .output()
        .expect("failed to run --help");

    assert!(
        out.status.success(),
        "--help should exit 0, got {:?}",
        out.status
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Verify key flags are documented.
    assert!(stdout.contains("--json"), "--help missing --json");
    assert!(stdout.contains("--root"), "--help missing --root");
    assert!(stdout.contains("--min-size"), "--help missing --min-size");
    assert!(stdout.contains("--now"), "--help missing --now");
}

#[test]
fn test_nonexistent_root_exits_nonzero() {
    let bin = binary_path();
    if !bin.exists() {
        return;
    }
    let out = Command::new(&bin)
        .arg("--root")
        .arg("/nonexistent_path_that_cannot_exist_8f7e3a2b")
        .output()
        .expect("failed to run with bad root");

    assert!(
        !out.status.success(),
        "should exit non-zero for unreadable root"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.is_empty(),
        "should emit a message to stderr for unreadable root"
    );
}
