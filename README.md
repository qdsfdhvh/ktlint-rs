# ktlint-rs

A fast Kotlin linter and formatter written in Rust — drop-in compatible with [ktlint](https://github.com/pinterest/ktlint).

[![CI](https://github.com/qdsfdhvh/ktlint-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/qdsfdhvh/ktlint-rs/actions)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://rust-lang.org)
[![Rules](https://img.shields.io/badge/rules-78-blue.svg)](https://github.com/qdsfdhvh/ktlint-rs)

## Features

- **78 rules** covering spacing, structure, imports, naming, wrapping, and KDoc
- **6x faster** than JVM ktlint via [rayon](https://github.com/rayon-rs/rayon) parallel processing
- **Drop-in CLI** compatible with ktlint arguments
- **.editorconfig** support with code style profiles (ktlint_official, android_studio, intellij_idea)
- **@Suppress** annotation support (`@Suppress("ktlint:rule-id")`)
- **4 reporters**: plain, JSON, SARIF, summary
- **Auto-fix** engine for spacing violations

## Quick Start

```bash
# Build from source
cargo build --release

# Lint a file
./target/release/ktlint path/to/File.kt

# Auto-fix violations
./target/release/ktlint --format path/to/File.kt

# Lint a directory (parallel)
./target/release/ktlint src/

# JSON output
./target/release/ktlint --reporter json src/
```

## Performance

Compared against [ktlint](https://github.com/pinterest/ktlint) JVM (v1.8.0).

| Project | Files | Lines | Violations (rs / JVM) | Time (rs / JVM) |
|---|---|---|---:|---:|
| [nowinandroid](https://github.com/android/nowinandroid) | 350 | 31,021 | 9,901 / 1,038 | **0.26s** / 6.71s |
| [compose-samples](https://github.com/android/compose-samples) (6 apps) | 380 | 46,586 | 10,752 / 13 | **0.30s** / 7.96s |
| [okhttp](https://github.com/square/okhttp) | 569 | 131,098 | 40,632 / 18 | **1.19s** / 11.5s |
| [androidx](https://github.com/androidx/androidx) (26 modules) | 1,271 | 266,549 | 86,591 / 33,731 | **1.07s** / 10.6s |
> Tested on Apple M2, release build with [rayon](https://github.com/rayon-rs/rayon).
> ktlint-rs 25-40x faster than JVM. Violation parity with `android_studio` profile in progress.
> ktlint-rs currently reports more violations than JVM; full rule parity with ktlint's `android_studio` profile is in progress.
## Configuration (.editorconfig)

```ini
[*.{kt,kts}]
ktlint_code_style = ktlint_official
indent_size = 4
indent_style = space
insert_final_newline = true
max_line_length = 0
ktlint_standard_no_wildcard_imports = enabled
```

## Rules

| Category | Count | Examples |
|---|---|---|
| Spacing | 17 | curly, operator, comma, paren, colon, dot, keyword, etc. |
| Structure | 27 | indent, trailing, blank-lines, max-line, trailing-comma, kdoc, etc. |
| Imports | 4 | wildcard, ordering, unused, etc. |
| Naming | 6 | class, function, property, filename, package, etc. |
| Wrapping | 7 | chain, multiline-if-else, try-catch, when, etc. |
| KDoc | 4 | formatting, no-empty, no-trailing, kdoc |
| Plus: Built-in (3) + Phase/Final (10) |
| **Total** | **78** | |
| Spacing | 25 | curly, operator, comma, paren, colon, dot, keyword, etc. |
| Structure | 30 | indent, trailing, blank-lines, max-line, trailing-comma, etc. |
| Imports | 5 | wildcard, ordering, unused, etc. |
| Naming | 8 | class, function, property, filename, package, etc. |
| Wrapping | 18 | chain, multiline-if-else, try-catch, when, etc. |
| KDoc | 6 | empty, first-line, trailing-space, no-blank, etc. |
| Comment | 5 | spacing, wrapping, block-comment, consecutive, etc. |
| Annotation | 3 | spacing, usage, @Suppress |

## Development

```bash
# Run tests (179+)
cargo test

# Build release
cargo build --release

# Run clippy
cargo clippy

# Format Rust code
cargo fmt
```

## License

MIT
