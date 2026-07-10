# AGENTS.md — ktlint-rs Development Guide

## Project Overview

**ktlint-rs** is a pure-Rust rewrite of [Pinterest ktlint](https://github.com/ktlint/ktlint),
a Kotlin linter and formatter. It aims for drop-in CLI compatibility and `.editorconfig`
support, with startup under 50ms and per-file lint under 5ms.

## Architecture

```
ktlint-rs/
├── src/
│   ├── main.rs          # Entry point: parse CLI → load config → discover → lint → report
│   ├── cli/mod.rs       # clap-based argument parsing (drop-in compat with ktlint)
│   ├── config/mod.rs    # .editorconfig loading + ktlint-specific properties
│   ├── discovery/mod.rs # File walker (respects .gitignore, .ktlintignore)
│   ├── parser/
│   │   ├── mod.rs       # KotlinParser: wraps tree-sitter-kotlin-sg
│   │   └── cst.rs       # CheckContext: offset→line:col, whitespace inspection
│   ├── rules/
│   │   ├── mod.rs       # Rule trait, RuleEngine, built-in simple rules
│   │   ├── spacing/     # Whitespace rules (curly, operator, comma, paren, colon, ...)
│   │   ├── structure/   # Indent, trailing space, blank lines, max-line-length, ...
│   │   ├── imports/     # Import ordering, no-wildcard, no-unused
│   │   ├── wrapping/    # Chain wrapping, argument wrapping (Phase 3)
│   │   └── naming/      # Class, function, property naming (Phase 3)
│   ├── formatter/mod.rs # Auto-fix engine (line-based, will extend to CST)
│   └── reporter/mod.rs  # Plain, JSON, SARIF, summary reporters
├── TASK_PLAN.md         # Detailed project plan with phases
└── Cargo.toml
```

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

## Current Status (2026-07-10)
- **Phase 0**: ✅ Infrastructure & skeleton (CLI, parser, config, discovery, reporters)
- **Phase 1**: 🟡 In progress (spacing: 9 rules ✅, structure: 7 rules ✅, imports: 3 rules ✅)
- **Phase 2**: ⬜ .editorconfig parity
- **Phase 3**: ⬜ Remaining rules (wrapping, naming)
- **Phase 4**: ⬜ Formatter & auto-fix engine
- **Phase 5**: ⬜ Advanced features (baselines, patterns, git hooks)
- **Phase 6**: ⬜ Testing & benchmarking
- **Phase 7**: ⬜ Distribution & docs

## Dependencies
- `tree-sitter` 0.24 — CST parsing
- `tree-sitter-kotlin-sg` 0.4 — Kotlin grammar
- `clap` 4 — CLI argument parsing
- `editorconfig` 1 — .editorconfig parsing
- `ignore` 0.4 — file discovery with .gitignore
- `colored` 3 — terminal output coloring
- `serde`/`serde_json` — JSON/SARIF reporters
- `anyhow` — error handling
- `log`/`env_logger` — logging

## Performance Targets
- Startup: <50ms
- Per-file lint: <5ms
- Full Kataris project (1377 files): <3s with `--format`
