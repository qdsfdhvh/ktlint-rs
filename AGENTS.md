# AGENTS.md — ktlint-rs Development Guide

## Project Overview

**ktlint-rs** is a pure-Rust rewrite of [Pinterest ktlint](https://github.com/ktlint/ktlint),
a Kotlin linter and formatter. It aims for drop-in CLI compatibility and `.editorconfig`
support, with startup under 50ms and per-file lint under 5ms.

**Performance Constraints (硬性要求):**
- **低内存**: 全项目 lint 后内存立即释放，无残留。禁止缓存大量文件内容。
- **低 CPU**: lint 完成后 CPU 归零，无后台线程/rayon pool 残留。
- **跑完即停**: 进程退出必须干净（exit 0/1/2），不允许 daemon 化或 event loop。
- **Rule 轻量**: 每个 rule 的 `check()` 必须 O(n) 且无副作用的纯函数。禁止规则内 I/O、网络、或全局状态。
- **二进制体量**: release binary < 15MB。
- **禁止 daemon**: 进程必须有明确的退出点（exit 0/1/2）。不允许常驻后台、server 模式、watch 模式、或任何形式的常驻进程。每次调用必须是一次性的：读文件 → lint → 输出 → 退出。
- **可缓存**: 允许使用 `.ktlint-rs/` 目录缓存解析结果或配置以加速重复运行，但缓存不得导致进程常驻。
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

### ⚠️ CRITICAL: Never commit directly to `master`

- **ALWAYS** create a feature branch before making changes.
- **ALWAYS** open a pull request — never push or merge directly to `master`.
- If you accidentally commit to master, immediately:
  1. `git branch <feature-branch>` to save your commits
  2. `git reset --hard <last-good-commit>` on master
  3. Push the feature branch and open a PR
- This is a hard requirement. No exceptions.

### Build & Test

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

### Branch & PR Policy
- **Branch protection on `main`**: requires a pull request before merging — no direct pushes.
- **Squash merge only**: all PRs are squash-merged into a single commit on `main`.
- Work on feature branches, open a PR when ready, and **never** try to push directly to `main`.

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

## Current Status (2026-07-15)
- **Phase 0**: ✅ Infrastructure & skeleton
- **Phase 1**: ✅ Core rules — 69 ktlint rules across all categories
- **Phase 2**: ✅ .editorconfig & config parity
- **Phase 3**: ✅ Parity tuning — 7 gaps fixed (PR #22)
- **Phase 4**: ✅ Formatter & auto-fix engine
- **Phase 5**: ✅ Advanced features (baselines, git hooks, YAML)
- **Phase 6**: ✅ Testing & benchmarking (330+ tests, CI, bench)
- **Phase 7**: ✅ Distribution & docs
- **Phase 8**: ✅ Registry + architecture refactor
- **Phase 9**: ✅ Unified config (namespace, category switches)
- **Phase 10**: ✅ CLI --ruleset integration
- **Phase 11**: ✅ detekt L0 rules (134/126)
- **Phase 12**: ⬜ blocked — Name resolution engine (~50 rules)
- **Phase 13**: ⬜ blocked — Type resolution bridge (~51 rules)

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
