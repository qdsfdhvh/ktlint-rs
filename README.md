
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
- **26x faster** than JVM ktlint (0.30s vs 7.96s on 380 files)
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

**Speed** (Apple M2 Pro, 380 Kotlin files, 46,586 lines — Google compose-samples):

| Metric | ktlint-rs | JVM ktlint |
|---|---|---|
| Time | **0.30s** | 7.96s (26⨉ slower) |
| Startup | <50ms | ~2s |
| Violations | **5,348** | 13 |
| Rules active | **78** | ~80 |

> ⚠️ ktlint-rs reports more violations because many newly-registered rules are strict
> by default. Tune via `.editorconfig`: `ktlint_standard_<id> = disabled`.

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
