# AGENTS.md — ktlint-rs Development Guide

> **Design goal**: Fast Kotlin pre-check tool for AI agents
> **Core principles**: Pure Rust, zero JVM, <1s scan
> **Details**: see [docs/DESIGN.md](docs/DESIGN.md)

## Project Overview

**ktlint-rs** is a pure-Rust rewrite of Pinterest ktlint and detekt.
It aims for drop-in CLI compatibility and `.editorconfig` support,
with startup under 50ms and per-file lint under 5ms.

**Performance Constraints (hard requirements):**
- **Low memory**: Free memory immediately after linting. No caching of file contents.
- **Low CPU**: CPU usage drops to zero after lint completes. No background threads/rayon pool.
- **Clean exit**: Process must exit cleanly (exit 0/1/2). No daemon or event loop.
- **Rule lightweight**: Each rule's `check()` must be O(n) and side-effect free.
- **Binary size**: release binary < 15MB.
- **No daemon**: Process must have a clear exit point. No server/watch mode.
- **Cache**: Uses `.cache/ktlint-rs/` directory for parser results/config to speed up repeated runs.

## Architecture

```
ktlint-rs/
├── src/
│   ├── rules/             # 69 ktlint + 147 detekt rules
│   ├── resolver/          # SymbolTable + TypeInfo extractor
│   ├── formatter/         # 31 auto-fix passes
│   ├── config/            # .editorconfig + YAML config
│   ├── cli/               # Clap CLI arguments
│   └── main.rs
├── tests/
│   ├── fixtures/          # Real-world Kotlin projects (nowinandroid, okhttp, etc.)
│   └── integration/       # Integration test binary
├── .github/workflows/     # CI pipeline
├── scripts/               # install.sh, install.ps1
└── docs/                  # DESIGN.md, LIMITATIONS.md, RULE_PLAN.md
```

## Agent LSP Configuration

Use **rust-analyzer** as the LSP. It is the official Rust language server:

- **Type checking**: `rust-analyzer diagnostics` or `cargo check`
- **Auto-completion**: rust-analyzer standard completion API
- **Go-to-definition**: rust-analyzer standard goto-definition
- **Find references**: rust-analyzer standard find-references

Quick commands for agent development:

```bash
# Fast type check (recommended, 2x faster than cargo check)
cargo check

# Strict lint (enable all warnings)
cargo clippy --all-features -- -D warnings

# Run all tests
cargo test --all-features

# Format code
cargo fmt --all

# Format check
cargo fmt --all -- --check
```

## TypeInfo Bridge (Phase 13)

Pure Rust type resolution via CST heuristics (`src/resolver/type_bridge.rs`):

- Extracts property types (`val x: String`)
- Extracts function return types (`fun foo(): Int`)
- Extracts constructor parameter types (`class Foo(val x: Int)`)
- Extracts parameter types (`fun bar(x: Int, y: String?)`)
- L2 rules use `check_with_symbols()` to access TypeInfo

## Constraints

- **Pure Rust**: Zero JVM / kotlinc / Gradle dependencies
- **Binary < 15MB**: Release mode
- **Startup < 50ms**: No daemon / rayon pool warmup
- **Memory release**: Immediate release after lint completes
- **No daemon**: Process must have clear exit point
