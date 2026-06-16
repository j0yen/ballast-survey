# ballast-survey

Read-only measurement layer for reclaimable disk weight. Walks a set of roots (default `~/wintermute`), finds reclaimable subtrees (`target/` dirs, cargo registry/git caches, `node_modules`/`.venv`, big `~/.cache` children), sizes each with last-modified mtime and age, and emits a structured JSON inventory sorted by reclaimable bytes. Deletes nothing. Used by ballast-reap and ballast-guard.

## Install

```
cargo install --path .
```

This installs the `ballast-survey` binary.

## Usage

```
ballast-survey [OPTIONS]

Options:
  --json              Emit structured JSON output (default: human-readable)
  --min-size <BYTES>  Skip entries below this size floor
  --now <RFC3339>     Override the current timestamp for deterministic age math
  --help              Print help
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
