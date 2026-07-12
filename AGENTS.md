# AGENTS.md — ktlint-rs Development Guide

## Project Overview

**ktlint-rs** is a pure-Rust rewrite of [Pinterest ktlint](https://github.com/ktlint/ktlint),
a Kotlin linter and formatter. It aims for drop-in CLI compatibility and `.editorconfig`
support, with startup under 50ms and per-file lint under 5ms.

## Architecture

```
ktlint-rs/
├── src/                    # Source code (see Architecture below)
├── tests/
│   ├── fixtures/           # Real-world Kotlin projects (cloned via scripts/setup-fixtures.sh)
│   └── integration/        # Integration test binary
├── scripts/
│   ├── bench.sh            # Performance benchmark + parity report
│   └── setup-fixtures.sh   # Shallow-clone test repos
├── skills/ktlint-rs/       # Published skill (npx skills add)
├── .agents/skills/         # Dev-time skill symlinked → skills/ktlint-rs/
├── task_plan.md            # Project plan (Phase status, parity gaps, priorities)
├── findings.md             # Research discoveries and root cause analysis
├── progress.md             # Session log
├── AGENTS.md               # This file
└── Cargo.toml

## Key Design Decisions

### Why CST (not AST)?
- tree-sitter-kotlin-sg produces a Concrete Syntax Tree that preserves ALL whitespace,
  comments, and formatting details. This is essential for a formatter that must fix
  violations while keeping non-violating code untouched.
- An AST would discard whitespace — we need it to check spacing.

### Rule Trait
```rust
pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;          // e.g. "standard:curly-spacing"
    fn auto_fixable(&self) -> bool;         // default: true
    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation>;
}
```

### CST Node Types (tree-sitter-kotlin-sg)
Key node kinds used by spacing rules:
- `{`, `}` — curly braces
- `=`, `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||` — operators
- `(`, `)` — parentheses
- `,` — comma
- `:` — colon
- `@`, `annotation` — annotations
- `comment`, `multiline_comment` — comments
- `function_declaration`, `class_declaration` — declarations
- `function_body`, `class_body` — bodies
- `value_arguments`, `class_parameters` — parameter lists

## Development Workflow

### Build & Test
```bash
# Build
cargo build

# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test curly_spacing
```

### Adding a New Rule
1. Create the rule file in the appropriate category directory
2. Implement the `Rule` trait
3. Add at least 2 tests: valid case (no violations) + invalid case
4. Register in `rules/mod.rs` → `RuleEngine::new()`
5. Add to TASK_PLAN.md if applicable

### Rule Pattern (CST-based)
```rust
impl Rule for MyRule {
    fn id(&self) -> &'static str { "standard:my-rule" }

    fn check(&self, tree: &Tree, source: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let bytes = source.as_bytes();
        self.walk(tree.root_node(), bytes, &mut violations);
        violations
    }
}

impl MyRule {
    fn walk(&self, node: Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        if node.kind() == "target_node_type" {
            self.check_node(&node, bytes, violations);
        }
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.walk(child, bytes, violations);
            }
        }
    }

    fn check_node(&self, node: &Node, bytes: &[u8], violations: &mut Vec<Violation>) {
        let pos = node.start_position();
        // Check whitespace around node using bytes
        // Push Violation if needed
    }
}
```

### Rule Pattern (line-based)
```rust
impl Rule for MyRule {
    fn id(&self) -> &'static str { "standard:my-rule" }

    fn check(&self, _tree: &Tree, source: &str) -> Vec<Violation> {
        source.lines()
            .enumerate()
            .filter(|(_, line)| /* condition */)
            .map(|(i, _)| Violation { /* ... */ })
            .collect()
    }
}
```

## Configuration System

### .editorconfig Properties

| Property | Values | Default |
|---|---|---|
| `ktlint_code_style` | `android_studio`, `intellij_idea`, `ktlint_official` | `ktlint_official` |
| `ktlint_experimental` | `enabled` | disabled |
| `ktlint_standard_<rule-id>` | `enabled`, `disabled` | `enabled` |
| `indent_size` | integer | `4` |
| `indent_style` | `space`, `tab` | `space` |
| `max_line_length` | integer | off |
| `insert_final_newline` | `true`, `false` | `true` |
| `trim_trailing_whitespace` | `true`, `false` | `true` |

### Code Style Profiles
- **ktlint_official**: All rules enabled (default)
- **android_studio**: Disables final-newline, no-wildcard-imports, import-ordering, trailing-comma, no-unused-imports
- **intellij_idea**: Disables no-wildcard-imports, import-ordering, trailing-comma

## Testing Strategy
- **Per-rule tests**: Each rule file has `#[cfg(test)] mod tests` with valid + invalid cases
- **Snapshot tests**: Compare ktlint-rs output vs JVM ktlint on known inputs
- **Integration test**: Run on real Kotlin project (kataris-app, 1377 files)
- **Benchmarks**: `cargo bench` for per-rule micro-benchmarks

## Current Status (2026-07-12)
- **Phase 0**: ✅ Infrastructure & skeleton
- **Phase 1**: ✅ Core rules — 65 rules across all categories
- **Phase 2**: 🟡 .editorconfig (code_style ✅, per-rule disable ⚠ pending wiring)
- **Phase 3**: 🟡 Parity tuning (indent, blank-line-before-declaration, etc.)
- **Phase 4**: ✅ Formatter & auto-fix engine
- **Phase 5**: ⬜ Advanced features
- **Phase 6**: 🟡 Testing & benchmarking (185 tests, parity analysis in progress)
- **Phase 7**: ⬜ Distribution & docs

See `task_plan.md` for detailed gap analysis and priority path.

## Dependencies
- `tree-sitter` 0.26 — CST parsing
- `tree-sitter-kotlin-sg` 0.4 — Kotlin grammar
- `clap` 4 — CLI argument parsing
- `editorconfig` 1 — .editorconfig parsing
- `ignore` 0.4 — file discovery with .gitignore
- `colored` 3 — terminal output coloring
- `serde`/`serde_json` — JSON/SARIF reporters
- `anyhow` — error handling
- `log`/`env_logger` — logging

## Performance (Apple M2, release)

| Project | Files | Lines | Time (rs / JVM) |
|---|---|---|---|
| nowinandroid | 350 | 31K | 0.26s / 6.71s (26x) |
| compose-samples | 380 | 47K | 0.30s / 7.96s (27x) |
| okhttp | 569 | 131K | 1.19s / 11.5s (10x) |
| androidx (26 mods) | 1,271 | 267K | 1.07s / 10.6s (10x) |

> Rayon parallel processing. Startup <2ms (debug).

## Skills

- `skills/ktlint-rs/SKILL.md` — Published skill for external users (`npx skills add`)
- `.agents/skills/ktlint-rs` — Symlink → `skills/ktlint-rs/`, for local dev-time agent context
