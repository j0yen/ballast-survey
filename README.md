# ballast-survey

A read-only inventory of reclaimable disk weight: it walks a set of roots, finds the subtrees that are safe-ish to delete — `target/` dirs, `node_modules/`, `.venv/`, `__pycache__/`, cargo caches, big `~/.cache` children — sizes each, and prints them sorted by bytes. It deletes nothing.

## Why it exists

Before you can reclaim disk, you have to know what is taking it and how risky each piece is to remove. `du` answers the first half and is silent on the second. A 14 GB `target/` for a crate whose binary you uninstalled months ago is trivially reclaimable; a 14 GB one for a project you built this morning is not. They look identical to `du`.

`ballast-survey` separates them. It classifies each reclaimable subtree by kind, and for Rust `target/` dirs it does one extra thing: it cross-references the installed binaries on the box against the target's mtime and assigns a `reap_safety` rank. A target whose binary is installed and newer is a fossil — safe to reclaim. The output is a ranked inventory other tools can act on, and a thing you never have to delete by hand from a guess.

The tool never modifies, writes to, or deletes any scanned path.

## Install

```sh
cargo install --path .
```

This installs the `ballast-survey` binary.

## Quickstart

Scan one or more roots (default `~/wintermute` if you pass none):

```sh
ballast-survey --root ~/wintermute
```

```text
ballast-survey  reclaimable: 7.63 MiB  entries: 3  scanned: 2026-06-16T12:00:00Z
PATH                                          SIZE       KIND               REAP_SAFETY  CRATE
/tmp/demo/proj/target                     4.77 MiB RustTarget        StaleUninstalled  proj
/tmp/demo/web/node_modules                1.91 MiB NodeModules                      -  web
/tmp/demo/py/.venv                      976.56 KiB Venv                             -  py
```

For machine-readable output — the form the rest of the ballast fleet consumes — add `--json`:

```sh
ballast-survey --root ~/wintermute --json
```

Each entry carries `path`, `kind`, `bytes`, `entries`, `mtime`, `age_days`, `crate_name`, and (for Rust targets) a `cloud_info` block with the `reap_safety` rank. The output is sorted by `bytes` descending under a `summary` header.

## What counts as reclaimable

| Kind | What it is |
|------|------------|
| `RustTarget` | Rust build artifacts (`target/` beside a `Cargo.toml`) |
| `NodeModules` | `node_modules/` |
| `Venv` | Python virtual environment (`.venv/`) |
| `Pycache` | Python bytecode cache (`__pycache__/`) |
| `CargoCache` | cargo registry or git-checkout cache |
| `CacheChild` | a top-level child of `~/.cache` over a size floor |

### Cloud-aware fossil classification

On by default. For each Rust `target/`, the survey looks up whether the crate's binary is installed and whether that binary is newer than the target. If so, the target is a fossil — its build output is superseded and safe to reclaim — and gets a `reap_safety` rank in its `cloud_info`. Pass `--no-cloudaware` to skip the cross-reference and emit the original v0.1 schema with no `cloud_info` fields.

If the installed binary's name differs from the crate's directory name, tell the survey with `--bin-name CRATE=BIN` (repeatable), e.g. `--bin-name wintermute-brain=wmd`.

## Flags

```text
-r, --root <DIR>          root to scan; repeatable (default: ~/wintermute)
    --json                emit JSON instead of a human table
    --min-size <SIZE>     skip entries below this floor (e.g. 100M, 1G)
    --now <RFC3339>       reference time for age math (deterministic tests)
    --no-cloudaware       disable fossil classification; emit the v0.1 schema
    --bin-name <CRATE=BIN> override the installed-binary name for a crate; repeatable
```

## Part of the ballast fleet

A family of read-mostly disk-health tools for the wintermute workspace. `ballast-survey` is the measurement layer the others build on — its `--json` output feeds trend, digest, and the reclamation tools.

| Tool | Job |
|------|-----|
| **`ballast-survey`** | Measure what is big right now ← you are here |
| [`ballast-trend`](https://github.com/j0yen/ballast-trend) | Measure what is growing and how fast (diffs survey snapshots) |
| [`ballast-guard`](https://github.com/j0yen/ballast-guard) | Watch usage against an SLO; log events; reclaim on opt-in |
| [`ballast-pilot`](https://github.com/j0yen/ballast-pilot) | Wire the guard to an hourly systemd timer |
| [`ballast-digest`](https://github.com/j0yen/ballast-digest) | Synthesize survey + trend + events into one ranked block |

## License

`MIT OR Apache-2.0`, at your option (per `Cargo.toml`).
