# ktlint-rs Task Plan

> A fast Kotlin linter & formatter in Rust — drop-in compatible with Pinterest ktlint CLI.
>
> **Status**: Phase 0 ✅ | Phase 1 🟡 | Phase 2 🟡 | Phase 3 ⬜ | Phase 4 ⬜

---

## Project Overview

**Goal**: Replace the JVM-based ktlint with a Rust-native binary that delivers
identical `.editorconfig`-driven linting output, supports auto-fix, and
integrates with existing Gradle/CI workflows — all with startup under 50ms
and per-file lint under 5ms.

**Reference**: Pinterest ktlint v1.8.0 (~80 standard rules, `--format`, `--baseline`, reporters)

**Current State** (2026-07-10):
- **62 rules** (spacing:17, structure:25, imports:4, naming:6, wrapping:7, built-in:4)
- 10-pass auto-fix engine with 59-71% violation reduction
- Rayon parallel processing: okhttp 0.34s / 525 files
- EditorConfig: indent_size, code_style, per-rule enable/disable
- @Suppress / @SuppressWarnings annotation support
- Reporters: plain, JSON, SARIF, summary
- GitHub Actions CI: test, clippy, fmt, self-lint
- AGENTS.md, .editorconfig, Pi skills
- **164 tests, all passing** ✅
---

## Phase 0 — Infrastructure & Skeleton ✅

**Hours**: 4 | **Status**: Done

- [x] `cargo init` with Cargo.toml dependencies
- [x] Module structure: `cli`, `config`, `discovery`, `parser`, `rules`, `formatter`, `reporter`
- [x] CLI argument parsing via clap (drop-in compatible with ktlint args)
- [x] `.editorconfig` loading stub
- [x] File discovery via `ignore` crate (respects `.gitignore` / `.ktlintignore`)
- [x] tree-sitter-kotlin-sg parser integration with test
- [x] Rule trait + engine dispatch

---

## Phase 1 — Core Spacing Rules (80% of Real-World Violations)

**Hours**: ~30 | **Target**: local `ktlint` drop-in works on Kataris with <5% false negatives

These rules account for ~80% of violations in real Kotlin codebases. Each uses
the tree-sitter CST directly (no need for high-level AST).

### 1.1 Spacing Rules (auto-fixable) — 10/12 ✅

| # | Rule | ktlint ID | Effort | Status |
|---|---|---|---|---|
| 1 | Curly spacing | `standard:curly-spacing` | 3h | ✅ |
| 2 | Operator spacing | `standard:op-spacing` | 3h | ✅ |
| 3 | Comma spacing | `standard:comma-spacing` | 2h | ✅ |
| 4 | Paren spacing | `standard:paren-spacing` | 2h | ✅ |
| 5 | Colon spacing | `standard:colon-spacing` | 2h | ✅ |
| 6 | Annotation spacing | `standard:annotation-spacing` | 2h | ✅ |
| 7 | Comment spacing | `standard:comment-spacing` | 2h | ✅ |
| 8 | Function return type spacing | `standard:function-return-type-spacing` | 2h | ✅ |
| 9 | Function start-of-body spacing | `standard:function-start-of-body-spacing` | 2h | ✅ |
| 10 | Class signature spacing | `standard:class-signature` | 2h | ⬜ |
| 12 | Argument list wrapping | `standard:argument-list-wrapping` | 2h | ⬜ |

### 1.2 Indentation & Whitespace (auto-fixable) — 8/8 ✅

| # | Rule | ktlint ID | Effort | Status |
|---|---|---|---|---|
| 13 | Indentation (4-space) | `standard:indent` | 4h | ✅ |
| 14 | No trailing spaces | `standard:no-trailing-spaces` | 1h | ✅ |
| 15 | Final newline | `standard:final-newline` | 1h | ✅ |
| 16 | No consecutive blank lines | `standard:no-consecutive-blank-lines` | 1h | ✅ |
| 17 | No blank line before rbrace | `standard:no-blank-line-before-rbrace` | 2h | ✅ |
| 18 | Max line length (120) | `standard:max-line-length` | 3h | ✅ |
| 19 | No empty file | `standard:no-empty-file` | 1h | ✅ |
| 20 | Trailing comma (configurable) | `standard:trailing-comma` | 3h | ✅ |

### 1.3 Imports (auto-fixable except wildcard) — 3/3 ✅

| # | Rule | ktlint ID | Effort | Status |
|---|---|---|---|---|
| 21 | No wildcard imports | `standard:no-wildcard-imports` | 2h | ✅ |
| 22 | Import ordering | `standard:import-ordering` | 4h | ✅ |
| 23 | No unused imports | `standard:no-unused-imports` | 3h | ✅ |

### 1.4 Phase 1 Validation

- [ ] Run `ktlint-rs` on kataris-app and compare output with JVM ktlint
- [ ] Diff <1% false positives on spacing rules
- [ ] `--format` produces identical output to `ktlint --format`
- [ ] `--format` on Kataris (1377 files): target <3s

---

## Phase 2 — .editorconfig & Config Parity

**Hours**: ~15 | **Target**: full `.editorconfig` compatibility with ktlint

### 2.1 Configuration Engine

### 2.1 Configuration Engine — 6/9 ✅

| # | Task | Effort | Status |
|---|---|---|---|
| 1 | Parse `[*.{kt,kts}]` section fully | 2h | ✅ (editorconfig crate) |
| 2 | `ktlint_code_style` (android_studio/intellij_idea/ktlint_official) | 2h | ✅ |
| 3 | Per-rule enable/disable: `ktlint_standard_<rule-id>` | 2h | ✅ |
| 4 | Rule-specific properties: `ktlint_function_naming_ignore_when_annotated_with`, etc. | 3h | ⬜ |
| 5 | `ij_kotlin_*` IntelliJ properties | 3h | ⬜ |
| 6 | `max_line_length`, `indent_size`, `indent_style`, `tab_width` | 2h | ✅ |
| 7 | `.editorconfig` file cascade (walk up directories) | 3h | ✅ (editorconfig crate) |
| 8 | CLI override for all config values | 2h | ✅ |
| 9 | `ktlint_experimental` flag for experimental rule gates | 1h | ✅ |
### 2.2 Code Style Profiles

| # | Profile | Diff from `ktlint_official` | Effort |
|---|---|---|---|
| 1 | `android_studio` | Disables ~5 rules, changes trailing-comma | 2h |
| 2 | `intellij_idea` | Disables ~3 rules | 2h |
| 3 | `ktlint_official` | Default, enables all rules | 1h |

---

## Phase 3 — Remaining Standard Rules

**Hours**: ~50 | **Target**: ~95% rule coverage vs JVM ktlint

### 3.1 Wrapping & Line Breaking (auto-fixable)

| # | Rule | Effort |
|---|---|---|
| 1 | Chain method continuation | 3h |
| 2 | Chain wrapping | 3h |
| 3 | Condition wrapping | 2h |
| 4 | Parameter list wrapping | 3h |
| 5 | Property wrapping | 2h |
| 6 | String template indent | 2h |
| 7 | Multiline if-else | 2h |
| 8 | Wrapping | 3h |

### 3.2 Naming (NOT auto-fixable — diagnostic only)

| # | Rule | ktlint ID | Effort |
|---|---|---|---|
| 1 | Class naming (PascalCase) | `standard:class-naming` | 2h |
| 2 | Function naming (camelCase) | `standard:function-naming` | 3h |
| 3 | Property naming (camelCase / SCREAMING_SNAKE) | `standard:property-naming` | 2h |
| 4 | Package name | `standard:package-name` | 2h |
| 5 | Filename matches top-level class | `standard:filename` | 3h |
| 6 | Backing property naming | `standard:backing-property-naming` | 3h |
| 7 | Enum entry name case | `standard:enum-entry-name-case` | 1h |

### 3.3 Structure & Semantics

| # | Rule | Auto-fix | Effort |
|---|---|---|---|
| 1 | Blank line before declaration | ⚠ ktlint_official only | 2h |
| 2 | Block comment initial star alignment | ✅ | 2h |
| 3 | No empty class body | ✋ | 1h |
| 4 | No empty first line in class body | ⚠ ktlint_official only | 2h |
| 5 | If-else bracing | ⚠ ktlint_official only | 2h |
| 6 | Mixed condition operators | ✅ | 2h |
| 7 | Multiline expression wrapping | ✅ | 2h |
| 8 | No semicolons | ✅ | 1h |
| 9 | String template | ✅ | 1h |
| 10 | Type argument list spacing | ✅ | 2h |
| 11 | Type parameter list spacing | ✅ | 2h |
| 12 | Discouraged comment location | ✋ | 2h |
| 13 | Value argument comment | ✋ | 2h |
| 14 | Value parameter comment | ✋ | 2h |
| 15 | Type argument comment | ✋ | 2h |
| 16 | Type parameter comment | ✋ | 2h |
| 17 | Spacing around range operator | ✅ | 1h |
| 18 | Nullable type parentheses | ✅ | 2h |
| 19 | Spacing around double colon | ✅ | 1h |
| 20 | Try-catch-finally spacing | ✅ | 2h |

### 3.4 Ktlint-Specific Features

| # | Feature | Effort |
|---|---|---|
| 1 | Legacy `// ktlint-disable` → `@Suppress` migration | 3h |
| 2 | `@Suppress("ktlint:standard:<rule-id>")` recognition | 2h |
| 3 | `@file:Suppress(...)` for file-level suppression | 2h |

---

## Phase 4 — Formatter & Auto-Fix Engine

**Hours**: ~25 | **Target**: `--format` produces clean output parsed by Kotlin compiler

### 4.1 Format Engine

| # | Task | Effort |
|---|---|---|
| 1 | CST-aware node replacement (not just line-based) | 6h |
| 2 | Whitespace-preserving tree printer | 6h |
| 3 | Sorted fix application (bottom-up to preserve offsets) | 3h |
| 4 | `.git-blame-ignore-revs` guidance output | 1h |
| 5 | Batch format with rayon parallelism | 3h |

### 4.2 Format Validation

| # | Task | Effort |
|---|---|---|
| 1 | Roundtrip test: parse → format → parse → no violations | 4h |
| 2 | Idempotency: format → format = no change | 2h |
| 3 | Kotlin compiler acceptance: formatted code compiles | 2h |
| 4 | Kataris project format: `cargo run -- -F` on 1377 files | 2h |

---

## Phase 5 — Advanced Features

**Hours**: ~30

### 5.1 Baselines

| # | Feature | Effort |
|---|---|---|
| 1 | XML baseline read/write (compatible with ktlint format) | 4h |
| 2 | `ktlint --baseline=ktlint-baseline.xml` (generate) | 2h |
| 3 | `ktlint --baseline=<file>` (check against) | 3h |

### 5.2 Patterns

| # | Feature | Effort |
|---|---|---|
| 1 | `--patterns-from-stdin` with newline/NUL delimiter | 2h |
| 2 | Glob negation patterns (`!**/Test.kt`) | 3h |
| 3 | `--stdin` + `--stdin-path` for editor integration | 2h |

### 5.3 Git Hooks

| # | Feature | Effort |
|---|---|---|
| 1 | `installGitPreCommitHook` command | 2h |
| 2 | `installGitPrePushHook` command | 2h |

### 5.4 CI Integration

| # | Feature | Effort |
|---|---|---|
| 1 | GitHub Actions annotation output (`::error file=...`) | 2h |
| 2 | Checkstyle reporter XML (compat with ktlint) | 3h |
| 3 | HTML reporter | 2h |
| 4 | `generateEditorConfig` command (generate .editorconfig template) | 2h |

---

## Phase 6 — Testing & Benchmarking

**Hours**: ~20

### 6.1 Test Suite

| # | Task | Effort |
|---|---|---|
| 1 | Per-rule test cases: valid + invalid code samples | 8h |
| 2 | Snapshot tests: known violation output matches JVM ktlint | 4h |
| 3 | Kataris project integration test | 3h |
| 4 | Regression test corpus from ktlint test suite | 3h |

### 6.2 Benchmarking

| # | Task | Effort |
|---|---|---|
| 1 | `cargo bench` micro-benchmarks per rule | 2h |
| 2 | ktlint-rs vs JVM ktlint on Kataris (cold & warm, full & incremental) | 1h |
| 3 | CI benchmark pipeline (publish results) | 2h |

---

## Phase 7 — Distribution & Docs

**Hours**: ~10

| # | Task | Effort |
|---|---|---|
| 1 | Homebrew formula | 2h |
| 2 | `cargo install ktlint-rs` | 1h |
| 3 | Pre-built binaries (GitHub Releases, aarch64 + x86_64, macOS/Linux) | 3h |
| 4 | Kataris project integration: replace or supplement JVM ktlint | 2h |
| 5 | README, CHANGELOG, contributing guide | 2h |

---

## Summary

| Phase | Scope | Est. Hours | Cumulative |
|---|---|---|---|
| 0 | Infrastructure & skeleton | 4 | 4 |
| 1 | Core spacing rules (72% of violations) | 30 | 34 |
| 2 | .editorconfig & config parity | 15 | 49 |
| 3 | Remaining standard rules | 50 | 99 |
| 4 | Formatter & auto-fix engine | 25 | 124 |
| 5 | Advanced features (baselines, hooks, CI) | 30 | 154 |
| 6 | Testing & benchmarking | 20 | 174 |
| 7 | Distribution & docs | 10 | 184 |
| **Total** | | **~184 hours** | |

**MVP cutoff**: Phase 1 + Phase 2 = ~49 hours → handles 80%+ of real-world violations
with proper .editorconfig support. Usable as a local pre-commit formatter.

**Full ktlint parity**: Phases 1-4 = ~124 hours → drop-in replacement for CI.

---

## Next Action

### Immediate (this session):
- [ ] Implement **curly-spacing** as the first real CST rule (pilot for the pattern)
- [ ] Implement **op-spacing** (high value, straightforward)
- [ ] Run `cargo build` to verify compilation

### First milestone:
- [ ] Run `ktlint-rs` on kataris-app → compare output with JVM ktlint
- [ ] Target: identify which rules Kataris actually needs (only ~5 disabled rules)
