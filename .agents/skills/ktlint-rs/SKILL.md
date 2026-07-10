# ktlint-rs Dev Skill

Use ktlint-rs to lint and format Kotlin code in this project.

## Quick Usage

```bash
# Build (once)
cd ktlint-rs && cargo build --release

# Lint all Kotlin files
target/release/ktlint src/

# Auto-fix spacing violations
target/release/ktlint --format src/

# Lint with JSON reporter
target/release/ktlint --reporter json path/to/File.kt
```

## When working on ktlint-rs itself

```bash
# Run tests
cargo test

# Run specific test
cargo test curly_spacing

# Add a new rule:
# 1. Create src/rules/<category>/new_rule.rs
# 2. Implement Rule trait
# 3. Register in src/rules/<category>/mod.rs
# 4. Register in src/rules/mod.rs → RuleEngine::new()
# 5. Add tests with #[cfg(test)]

# Format Rust code before commit
cargo fmt
cargo clippy
```

## Rule Types

| Category | Directory | Pattern |
|---|---|---|
| Spacing | `rules/spacing/` | CST-based byte inspection |
| Structure | `rules/structure/` | Line-based or CST walk |
| Imports | `rules/imports/` | Line-based |
| Naming | `rules/naming/` | Line-based keyword extraction |
| Wrapping | `rules/wrapping/` | Multi-line pattern matching |

## Current State

- **100 rules** registered in RuleEngine
- **164 tests**, all passing
- 0.34s on okhttp (525 files) via rayon parallel
- EditorConfig: indent_size, code_style, per-rule enable/disable
