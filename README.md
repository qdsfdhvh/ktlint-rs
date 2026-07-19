
# ktlint-rs

A blazingly fast pure-Rust [ktlint](https://github.com/pinterest/ktlint) — Kotlin linter & formatter.

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rules](https://badgen.net/badge/rules/226/blue)](https://github.com/qdsfdhvh/ktlint-rs)
[![Tests](https://badgen.net/badge/tests/447/green)](https://github.com/qdsfdhvh/ktlint-rs)
[![CI](https://github.com/qdsfdhvh/ktlint-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/qdsfdhvh/ktlint-rs/actions)

## Why

Kotlin tooling in Rust — startup under 50ms, lint per file under 5ms.

- **226 rules** across 12 categories — zero JVM dependency
- **31 auto-fix passes** for spacing, wrapping, indentation
- **Drop-in CLI** compatible with JVM ktlint
- **6 reporters**: plain, JSON, SARIF, checkstyle, HTML, markdown
- **`.editorconfig`** support with ktlint properties
- **Zero JVM**: pure Rust, single binary < 12MB

## Install

### macOS / Linux

```bash
# Preferred: cargo install from git (needs Rust toolchain)
cargo install --git https://github.com/qdsfdhvh/ktlint-rs --tag v0.1.0 ktlint-rs

# Or: pre-built binary
curl -fsSL https://github.com/qdsfdhvh/ktlint-rs/releases/latest/download/install.sh | bash
```

### Windows

```powershell
iwr -useb https://github.com/qdsfdhvh/ktlint-rs/releases/latest/download/install.ps1 | iex
```

## Quick Start

```bash
# Lint
ktlint-rs **/*.kt

# Auto-fix
ktlint-rs --format **/*.kt

# JSON output
ktlint-rs --reporter json **/*.kt

# Detekt rules only
ktlint-rs --ruleset detekt
```

> Full CLI reference: **[docs/command.md](docs/command.md)**  
> Design philosophy: **[docs/DESIGN.md](docs/DESIGN.md)**  
> Performance data: **[docs/PERFORMANCE.md](docs/PERFORMANCE.md)**

## Performance

| Project | Files | ktlint-rs | JVM ktlint | Speedup |
|---|---|---|---|---|
| nowinandroid | 350 | **0.29s** | 7.1s | **24×** |
| okhttp | 569 | 0.66s | 11.5s | **17×** |

## Rule Coverage

| Category | ktlint | detekt | Total |
|---|---|---|---|
| Spacing | 17 | - | 17 |
| Structure | 29 | — | 29 |
| Imports | 4 | — | 4 |
| Naming | 6 | 20 | 26 |
| Wrapping | 8 | - | 8 |
| Empty-blocks | — | 14 | 14 |
| Complexity | — | 15 | 15 |
| Style | — | 61 | 61 |
| Comments | — | 9 | 9 |
| Exceptions | — | 9 | 9 |
| Potential-bugs | — | 18 | 18 |
| **Total** | **78** | **148** | **226** |

## Advanced Features

### Baselines

```bash
ktlint-rs --create-baseline --baseline baseline.xml
ktlint-rs --baseline baseline.xml
```

### .editorconfig

```ini
[*.kt]
indent_size = 4
max_line_length = 140

# Disable specific rules
ktlint_standard_no_wildcard_imports = disabled
```

## Development

```bash
cargo check              # Fast type check
cargo test --all-features # 447 tests
cargo fmt --all          # Format code
cargo build --release    # Release build
```

## License

[MIT](LICENSE)
