
# ktlint-rs

A blazingly fast pure-Rust [ktlint](https://github.com/pinterest/ktlint) — Kotlin linter & formatter.

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![ktlint](https://img.shields.io/badge/ktlint-1.8.0-green.svg)](https://github.com/pinterest/ktlint)
[![Rules](https://img.shields.io/badge/rules-78-blue.svg)](https://github.com/qdsfdhvh/ktlint-rs)
[![Tests](https://img.shields.io/badge/tests-193-green.svg)](https://github.com/qdsfdhvh/ktlint-rs)
[![CI](https://github.com/qdsfdhvh/ktlint-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/qdsfdhvh/ktlint-rs/actions)

## Why

Kotlin tooling in Rust — startup under 50ms, lint per file under 5ms. Drop-in compatible with the JVM ktlint CLI.

- **78 rules** covering spacing, structure, imports, naming, wrapping, and KDoc
- **340-rule parity tracker** — [docs/RULE_PLAN.md](docs/RULE_PLAN.md) covers JVM ktlint (105) + detekt (217)
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

| demo-gradle | 8 | 9ms / 2.11s | **235×** |

> Full per-project breakdown: **[docs/PERFORMANCE.md](docs/PERFORMANCE.md)** — violations, rules, files hit, throughput, detekt comparison.

## Advanced Features

### Baselines
Suppress known violations in legacy projects:
```bash
ktlint-rs --create-baseline --baseline baseline.xml  # generate
ktlint-rs --baseline baseline.xml                     # lint with suppression
```

### Git Hooks
Auto-lint staged files before each commit:
```bash
ktlint-rs --install-git-hook    # install pre-commit hook
ktlint-rs --uninstall-git-hook  # remove it
```

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
