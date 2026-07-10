# Rust Dev Skill

Use rust-analyzer LSP, clippy, and rustfmt for faster, higher-quality Rust development.

## Prerequisites

```bash
# One-time install
rustup component add rust-analyzer rustfmt clippy
```

## Workflow

### When writing/changing code
1. **rustfmt** before committing: `cargo fmt`
2. **clippy** for lint warnings: `cargo clippy`
3. **rust-analyzer** for symbol lookup — use it instead of grep for type-aware queries

### When debugging
- Run `cargo check` for fast compilation errors (no codegen)
- Run `cargo test` for test failures
- Use `cargo expand` to see macro expansions

### Cargo commands reference

| Command | Purpose |
|---|---|
| `cargo fmt` | Auto-format code |
| `cargo fmt -- --check` | Check formatting without changing |
| `cargo clippy` | Run linter |
| `cargo clippy -- -D warnings` | Treat warnings as errors |
| `cargo check` | Fast compile check (no binary) |
| `cargo build` | Full build |
| `cargo test` | Run all tests |
| `cargo test <name>` | Run specific test |
| `cargo test -- --nocapture` | Show test output |
| `cargo doc --no-deps --open` | Open docs |
| `cargo bench` | Run benchmarks |

## Project-specific notes (ktlint-rs)

- Rust edition 2021, MSRV: stable
- tree-sitter 0.24 + tree-sitter-kotlin-sg 0.4
- All rules in `src/rules/` with per-file `#[cfg(test)]` tests
- Add new rules in `rules/<category>/` then register in `rules/mod.rs`
