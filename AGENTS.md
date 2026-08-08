# AGENTS.md — ktlint-rs Development Guide

> **Design goal**: Fast Kotlin pre-check tool for AI agents
> **Core principles**: Pure Rust, zero JVM, <1s scan
> **Details**: see [docs/DESIGN.md](docs/DESIGN.md)

## Rules — STOP and read before acting

Scan [`.agents/rules/INDEX.md`](.agents/rules/INDEX.md) at task start. If a
row matches, **read that rule file before coding**. The non-negotiable ones:

- **Any git write** → `git-workflow/RULE.md`
- **Installing/updating the local binary or releasing** → `local-install/RULE.md` (release-only installs)
- **Rule/formatter changes** → `parity/RULE.md` (oracle + consumer-corpus gates)
- **Hot-path changes** → `performance/RULE.md` (perf gates)
- **Touching a consumer project** (kataris-app, ktor, …) → `scope/RULE.md` (validation only)

## Project

**ktlint-rs** is a pure-Rust rewrite of Pinterest ktlint and detekt:
drop-in CLI compatibility, `.editorconfig` support, startup <50ms,
per-file lint <5ms, binary <30MB, clean exit (no daemon). Hard runtime
gates are enforced by `scripts/perf-gates.sh` (see `performance/RULE.md`).

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
cargo check                                # fast type check
cargo clippy --all-features -- -D warnings # strict lint
cargo test --all-features                  # all tests
cargo fmt --all && cargo fmt --all -- --check
```

## TypeInfo Bridge (Phase 13)

Pure Rust type resolution via CST heuristics (`src/resolver/type_bridge.rs`):

- Extracts property types (`val x: String`)
- Extracts function return types (`fun foo(): Int`)
- Extracts constructor parameter types (`class Foo(val x: Int)`)
- Extracts parameter types (`fun bar(x: Int, y: String?)`)
- L2 rules use `check_with_symbols()` to access TypeInfo
