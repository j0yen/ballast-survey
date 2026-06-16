//! ballast-survey CLI entry point.

use anyhow::{Context, Result};
use ballast_survey::roots::expand_roots;
use chrono::{DateTime, Utc};
use clap::Parser;
use humansize::{BINARY, format_size};

/// Read-only inventory of reclaimable disk weight.
///
/// Walks one or more root directories, finds reclaimable subtrees (Rust
/// target/ dirs, node_modules/, .venv/, __pycache__/, cargo caches), sizes
/// each, and emits a structured inventory sorted by reclaimable bytes.
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
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
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
        args.root.clone()
    };

    let roots = expand_roots(&raw_roots).context("failed to resolve scan roots")?;

    // Parse min-size.
    let min_bytes = match args.min_size {
        Some(ref s) => parse_size(s).with_context(|| format!("invalid --min-size value: {s}"))?,
        None => 0,
    };

    // Run the survey.
    let output = ballast_survey::survey(&roots, min_bytes, now)?;

    if args.json {
        let json = serde_json::to_string_pretty(&output)
            .context("failed to serialize survey output")?;
        #[allow(clippy::print_stdout)]
        {
            println!("{json}");
        }
    } else {
        print_table(&output);
    }

    Ok(())
}

/// Parse a human-readable size string (e.g. "100M", "1G") into bytes.
fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("size string is empty");
    }

    let (digits, suffix) = s.split_at(s.len() - s.chars().rev().take_while(|c| c.is_alphabetic()).count());
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

fn print_table(output: &ballast_survey::Output) {
    let s = &output.summary;
    #[allow(clippy::print_stdout)]
    {
        println!(
            "ballast-survey  reclaimable: {}  entries: {}  scanned: {}",
            format_size(s.reclaimable_bytes, BINARY),
            s.entry_count,
            s.scanned_at.format("%Y-%m-%dT%H:%M:%SZ"),
        );
        println!(
            "{:<60} {:>12} {:>10} {:>8}  {}",
            "PATH", "SIZE", "KIND", "AGE", "CRATE"
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
    }
}
