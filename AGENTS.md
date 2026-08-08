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
- **Binary size**: release binary < 30MB.
- **No daemon**: Process must have a clear exit point. No server/watch mode.
- **Cache**: Uses `.cache/ktlint-rs/` directory for parser results/config to speed up repeated runs.

## Architecture

```
ktlint-rs/
├── src/
│   ├── rules/             # 78 ktlint + 148 detekt rules (226 total)
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

## Git Workflow

- **Always branch + PR**: never push directly to `main`. Create a feature branch (`feat/...`, `fix/...`, `docs/...`, `ci/...`), push, and open a PR.
- Docs-only changes (`**.md`, `docs/**`) skip CI via `paths-ignore` — no need to wait for build.
- Commit messages: conventional commits (`feat:`, `fix:`, `docs:`, `ci:`, `refactor:`, `test:`).
## TypeInfo Bridge (Phase 13)

Pure Rust type resolution via CST heuristics (`src/resolver/type_bridge.rs`):

- Extracts property types (`val x: String`)
- Extracts function return types (`fun foo(): Int`)
- Extracts constructor parameter types (`class Foo(val x: Int)`)
- Extracts parameter types (`fun bar(x: Int, y: String?)`)
- L2 rules use `check_with_symbols()` to access TypeInfo

## Constraints

- **Pure Rust**: Zero JVM / kotlinc / Gradle dependencies
- **Binary < 30MB**: Release mode
- **Startup < 50ms**: No daemon / rayon pool warmup
- **Memory release**: Immediate release after lint completes
- **No daemon**: Process must have clear exit point

## Local install discipline (hard rule)

The local `ktlint-rs` on PATH (`~/.cargo/bin/ktlint-rs`) must be installed
**only from a GitHub release** — never by copying a local build
(`target/release/ktlint-rs`) or by `cargo install` from a working tree.

- Official install: `scripts/install-release.sh [tag] [install-dir]` (downloads
  the per-platform asset from `github.com/qdsfdhvh/ktlint-rs/releases`).
- Before tagging: bump `Cargo.toml`/`Cargo.lock`, merge the version PR with a
  green CI (including perf + mutation gates), then `git tag vX.Y.Z` and
  `gh release create` — the release workflow uploads linux/macOS/Windows
  binaries automatically.
- Local `target/release` builds are for development/testing only and must
  never replace the PATH binary.
- Verify after install: `ktlint-rs --version` must equal the released tag.
