//! ballast-survey CLI entry point.

use anyhow::{Context, Result};
use ballast_survey::roots::expand_roots;
use ballast_survey::{BinNameOverrides, SurveyOptions};
use chrono::{DateTime, Utc};
use clap::Parser;
use humansize::{BINARY, format_size};
use std::collections::HashMap;

/// Read-only inventory of reclaimable disk weight.
///
/// Walks one or more root directories, finds reclaimable subtrees (Rust
/// `target/` dirs, `node_modules/`, `.venv/`, `__pycache__/`, cargo caches), sizes
/// each, and emits a structured inventory sorted by reclaimable bytes.
///
/// By default, Rust `target/` entries are cross-referenced against installed
/// binaries to classify fossil targets (safe to reclaim). Pass `--no-cloudaware`
/// to suppress this and reproduce the v0.1 output schema.
///
/// The tool never modifies, writes to, or deletes any scanned path.
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Root directory to scan. Can be specified multiple times.
    /// Defaults to ~/wintermute if not specified.
    #[arg(long, short = 'r', value_name = "DIR")]
    root: Vec<String>,

    /// Emit machine-readable JSON instead of a human table.
    #[arg(long)]
    json: bool,

    /// Exclude entries smaller than this size (e.g. 100M, 1G).
    #[arg(long, value_name = "SIZE")]
    min_size: Option<String>,

    /// Reference time for age computation (RFC 3339). Defaults to the current
    /// wall-clock time. Use this flag in scripts or tests for deterministic output.
    #[arg(long, value_name = "RFC3339")]
    now: Option<String>,

    /// Disable cloud-aware fossil classification.
    ///
    /// When set, the output will exactly match the v0.1 schema: no `cloud_info`
    /// fields are emitted on any entry.  Use this when you only need the raw size
    /// ranking without the installed-binary cross-reference overhead.
    #[arg(long)]
    no_cloudaware: bool,

    /// Override the installed-binary name for a crate (KEY=VALUE pairs).
    ///
    /// Use when the installed binary name differs from the crate directory name,
    /// e.g. `--bin-name wintermute-brain=wmd`.  Can be repeated.
    #[arg(long, value_name = "CRATE=BIN", value_parser = parse_bin_override)]
    bin_name: Vec<(String, String)>,
}

/// Parse a `CRATE=BIN` key-value pair for `--bin-name`.
fn parse_bin_override(s: &str) -> std::result::Result<(String, String), String> {
    let (k, v) = s.split_once('=').ok_or_else(|| {
        format!("expected CRATE=BIN, got: {s}")
    })?;
    if k.is_empty() || v.is_empty() {
        return Err(format!("both crate name and binary name must be non-empty: {s}"));
    }
    Ok((k.to_owned(), v.to_owned()))
}

fn main() {
    if let Err(e) = run() {
        #[allow(clippy::print_stderr)]
        {
            eprintln!("error: {e:#}");
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Resolve `now` — wall-clock at the CLI boundary only.
    let now: DateTime<Utc> = match args.now {
        Some(ref s) => DateTime::parse_from_rfc3339(s)
            .with_context(|| format!("invalid --now value: {s}"))?
            .into(),
        None => Utc::now(),
    };

    // Resolve roots.
    let raw_roots: Vec<String> = if args.root.is_empty() {
        vec!["~/wintermute".to_owned()]
    } else {
        args.root
    };

    let roots = expand_roots(&raw_roots).context("failed to resolve scan roots")?;

    // Parse min-size.
    let min_bytes = match args.min_size {
        Some(ref s) => parse_size(s).with_context(|| format!("invalid --min-size value: {s}"))?,
        None => 0,
    };

    // Build survey options.
    let bin_name_overrides: BinNameOverrides =
        args.bin_name.into_iter().collect::<HashMap<_, _>>();
    let opts = SurveyOptions {
        no_cloudaware: args.no_cloudaware,
        bin_name_overrides,
    };

    // Run the survey.
    let output = ballast_survey::survey_with_options(&roots, min_bytes, now, &opts);

    if args.json {
        let json = serde_json::to_string_pretty(&output)
            .context("failed to serialize survey output")?;
        #[allow(clippy::print_stdout)]
        {
            println!("{json}");
        }
    } else {
        print_table(&output, args.no_cloudaware);
    }

    Ok(())
}

/// Parse a human-readable size string (e.g. "100M", "1G") into bytes.
fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("size string is empty");
    }

    let alpha_count = s.chars().rev().take_while(|c| c.is_alphabetic()).count();
    let (digits, suffix) = s.split_at(s.len() - alpha_count);
    let value: u64 = digits.trim().parse().with_context(|| format!("not a number: {digits}"))?;

    let multiplier: u64 = match suffix.to_ascii_uppercase().as_str() {
        "" | "B" => 1,
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        "T" | "TB" | "TIB" => 1024_u64 * 1024 * 1024 * 1024,
        other => anyhow::bail!("unknown size suffix: {other}"),
    };

    Ok(value * multiplier)
}

fn print_table(output: &ballast_survey::Output, no_cloudaware: bool) {
    use ballast_survey::classify::EntryKind;

    let s = &output.summary;
    #[allow(clippy::print_stdout)]
    {
        println!(
            "ballast-survey  reclaimable: {}  entries: {}  scanned: {}",
            format_size(s.reclaimable_bytes, BINARY),
            s.entry_count,
            s.scanned_at.format("%Y-%m-%dT%H:%M:%SZ"),
        );

        if no_cloudaware {
            println!(
                "{:<60} {:>12} {:>10} {:>8}  CRATE",
                "PATH", "SIZE", "KIND", "AGE",
            );
            println!("{}", "-".repeat(100));
            for e in &output.entries {
                println!(
                    "{:<60} {:>12} {:>10} {:>6}d  {}",
                    e.path.display(),
                    format_size(e.bytes, BINARY),
                    format!("{:?}", e.kind),
                    e.age_days,
                    e.crate_name,
                );
            }
        } else {
            println!(
                "{:<60} {:>12} {:>10} {:>8}  {:>20}  CRATE",
                "PATH", "SIZE", "KIND", "AGE", "REAP_SAFETY",
            );
            println!("{}", "-".repeat(115));
            for e in &output.entries {
                let safety = if e.kind == EntryKind::RustTarget {
                    e.cloud_info
                        .as_ref()
                        .map_or_else(|| "-".to_owned(), |ci| format!("{:?}", ci.reap_safety))
                } else {
                    "-".to_owned()
                };
                println!(
                    "{:<60} {:>12} {:>10} {:>6}d  {:>20}  {}",
                    e.path.display(),
                    format_size(e.bytes, BINARY),
                    format!("{:?}", e.kind),
                    e.age_days,
                    safety,
                    e.crate_name,
                );
            }
        }
    }
}
