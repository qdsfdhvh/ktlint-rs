
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
ktlint **/*.kt

# Auto-fix
ktlint --format **/*.kt

# JSON output
ktlint --reporter=json **/*.kt
```

## Performance

**Speed** (Apple M2 Pro):

| Project | Files | Lines | ktlint-rs | JVM | Time (rs / JVM) |
|---|---|---|---|---|---|
| nowinandroid | 350 | 31,021 | 9,901 | 1,038 | 0.26s / 6.71s |
| compose-samples | 380 | 46,586 | **5,348** | 13 | 0.30s / 7.96s |
| okhttp | 569 | 131,098 | 40,632 | 18 | 1.19s / 11.5s |
| androidx | 1,271 | 266,549 | 86,591 | 33,731 | 1.07s / 10.6s |

> ⚠️ compose-samples refreshed 2026-07-12 (post dedup fix).
> nowinandroid / okhttp / androidx numbers from v0.2.0 — pending re-benchmark.

## Rule Coverage

| Category | Count | Examples |
|---|---|---|
| Spacing | 17 | curly, operator, comma, paren, colon, dot, keyword |
| Structure | 27 | indent, trailing, blank-lines, max-line, kdoc |
| Imports | 4 | wildcard, ordering, unused |
| Naming | 6 | class, function, property, filename, package |
| Wrapping | 7 | chain, multiline-if-else, try-catch, when |
| KDoc | 4 | formatting, no-empty, no-trailing |
| Plus | 13 | Built-in (3) + Phase/Final rules |
| **Total** | **78** | |

See [TASK_PLAN.md](TASK_PLAN.md) for full parity tracking.

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
cargo test

# Run on fixtures
cargo run -- tests/fixtures/compose-samples/

# Build release
cargo build --release
```

## License

MIT
