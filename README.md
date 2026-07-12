# ktlint-rs

A fast Kotlin linter and formatter written in Rust — drop-in compatible with [ktlint](https://github.com/pinterest/ktlint).

[![CI](https://github.com/qdsfdhvh/ktlint-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/qdsfdhvh/ktlint-rs/actions)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://rust-lang.org)
[![Rules](https://img.shields.io/badge/rules-100-blue.svg)](https://github.com/qdsfdhvh/ktlint-rs)

## Features

- **100 rules** covering spacing, structure, imports, naming, wrapping, and KDoc
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

| Project | Files | Lines | Time |
|---|---|---|---|
| nowinandroid | 350 | 31,021 | **0.58s** |
| compose-samples | 380 | 46,586 | **0.61s** |
| okhttp | 569 | 131,098 | **0.87s** |
| androidx (26 modules) | 1,271 | 532,795 | **0.86s** |

> Tested on Apple M3 Pro, release build with [rayon](https://github.com/rayon-rs/rayon) parallel processing.
> AndroidX benchmark covers 26 self-contained modules (activity, fragment, compose-runtime, etc.).
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
# Run tests (164+)
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
