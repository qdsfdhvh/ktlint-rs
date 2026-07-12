
# ktlint-rs

A blazingly fast pure-Rust [ktlint](https://github.com/pinterest/ktlint) — Kotlin linter & formatter.

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![ktlint](https://img.shields.io/badge/ktlint-1.6.0-green.svg)](https://github.com/pinterest/ktlint)
[![Rules](https://img.shields.io/badge/rules-78-blue.svg)](https://github.com/qdsfdhvh/ktlint-rs)
[![Tests](https://img.shields.io/badge/tests-179+-green.svg)](https://github.com/qdsfdhvh/ktlint-rs)

## Why

Kotlin tooling in Rust — startup under 50ms, lint per file under 5ms. Drop-in compatible with the JVM ktlint CLI.

- **78 rules** covering spacing, structure, imports, naming, wrapping, and KDoc
- **Drop-in CLI** compatible with JVM ktlint
- **4 reporters**: plain, JSON, SARIF, summary
- **`.editorconfig`** support with ktlint properties
- **Auto-fix** for fixable violations (`--format`)

## Quick Start

```bash
# Install
cargo install ktlint-rs

# Lint
ktlint-rs **/*.kt

# Auto-fix
ktlint-rs --format **/*.kt

# JSON output
ktlint-rs --reporter=json **/*.kt
```

## Performance

**Speed** (Apple M2, release build, rayon):

| Project | Files | Lines | Time (rs / JVM) | Speedup |
|---|---:|---:|---:|---:|
| nowinandroid | 350 | 31,021 | 0.23s / 6.94s | **30x** |
| compose-samples (6 apps) | 380 | 46,586 | 0.31s / 6.93s | **22x** |
| okhttp | 569 | 131,098 | 1.25s / 8.16s | **7x** |
| androidx (26 modules) | 1,271 | 266,549 | 1.07s / 10.6s | **10x** |
| demo-gradle | 8 | 162 | 0.01s / 1.85s | **155x** |

> Benchmarked 2026-07-12 with `scripts/bench.sh --release`.
> Violation parity with JVM under `ktlint_official` code style is in progress.
> See `task_plan.md` for detailed gap analysis.

## Rule Coverage

| Category | Count | Examples |
|---|---|---|
| Spacing | 17 | curly, operator, comma, paren, colon, dot, keyword, annotation, modifier-order |
| Structure | 28 | indent, trailing, blank-lines, max-line, trailing-comma, kdoc |
| Imports | 4 | wildcard, ordering, unused |
| Naming | 6 | class, function, property, filename, package |
| Wrapping | 7 | chain, multiline-if-else, try-catch, when |
| KDoc | 3 | formatting, no-empty, no-trailing |
| **Total** | **65** | |


## .editorconfig Support

```ini
[*.kt]
ktlint_code_style = android_studio
indent_size = 4
max_line_length = 140
insert_final_newline = true

# Disable specific rules
ktlint_standard_no_wildcard_imports = disabled
ktlint_standard_trailing_comma = disabled
```

## Development

```bash
# Build
cargo build

# Run tests (179+)
cargo test # 185 tests

# Run on fixtures
cargo run -- tests/fixtures/compose-samples/

# Build release
cargo build --release
```

## License

MIT
