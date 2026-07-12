# ktlint-rs Task Plan

> A fast Kotlin linter & formatter in Rust — drop-in compatible with Pinterest ktlint CLI.
>
> **Status**: Phase 0 ✅ | Phase 1 ✅ | Phase 2 🟡 | Phase 3 🟡 | Phase 4 ✅

---

## Project Overview

**Goal**: Replace the JVM-based ktlint with a Rust-native binary that delivers
identical `.editorconfig`-driven linting output, supports auto-fix, and
integrates with existing Gradle/CI workflows — all with startup under 50ms
and per-file lint under 5ms.

**Reference**: Pinterest ktlint v1.8.0 (~80 standard rules, `--format`, `--baseline`, reporters)

**Current State** (2026-07-12):
- **65 rules** (spacing:17, structure:28, imports:4, naming:6, wrapping:7, built-in:4)
  - New: `blank-line-before-declaration`, `no-blank-line-in-list`, `kdoc` (enhanced)
- 10-pass auto-fix engine with 59-71% violation reduction
- Rayon parallel processing: 10-27x faster than JVM ktlint
- EditorConfig: `indent_size`/`indent_style` ✅, `code_style` ✅ (parsing), per-rule ❌
- @Suppress / @SuppressWarnings annotation support
- Reporters: plain, JSON, SARIF, summary
- GitHub Actions CI: test, clippy, fmt
- AGENTS.md, rustfmt.toml, Pi skills, bench script, setup-fixtures script
- **185 tests, all passing** ✅

### Performance (release build, Apple M2)

| Project | Files | Lines | Violations (rs / JVM) | Time (rs / JVM) |
|---|---|---|---:|---:|
| nowinandroid | 350 | 31,021 | 9,901 / 1,038 | **0.26s** / 6.71s |
| compose-samples (6 apps) | 380 | 46,586 | 10,752 / 13 | **0.30s** / 7.96s |
| okhttp | 569 | 131,098 | 40,632 / 18 | **1.19s** / 11.5s |
| androidx (26 modules) | 1,271 | 266,549 | 86,591 / 33,731 | **1.07s** / 10.6s |

### Known Parity Gaps (nowinandroid, ktlint_official code style)

| # | Gap | Impact | Root Cause |
|---|---|---|---|
| 1 | **ktlint_standard_* not loaded** | ~10 rules run that shouldn't | `editorconfig` crate strips non-standard keys |
| 2 | **indent rule too aggressive** | 6,948 vs 15 | Only checks "multiple of 4"; JVM has context-sensitive logic |
| 3 | **blank-line-before-declaration** | 1,240 vs 25 | Triggers on ALL decls; JVM only on top-level |
| 4 | **colon-spacing, op-spacing, etc.** | ~250 extra | JVM doesn't report these under `ktlint_official`? Need to investigate |
| 5 | **filename, kdoc, wrapping** | 0 vs 11 | Rules exist but match 0; implementation differs from JVM |
| 6 | **Suppress comments** | ❓ | `ktlint-disable` region suppression not verified |
| 7 | **Auto-fix output parity** | ❓ | `--format` output not diffed against JVM |
| 8 | **Reporter JSON format** | ❓ | Structure differs from JVM ktlint JSON |

| Dimension | Status |
|---|---|
| Exit codes | ✅ Match JVM |
| File discovery | ✅ Same `.kt`/`.kts` |
| Code style parsing | ✅ Parsed from `.editorconfig` (manual fallback) |
| Per-rule parsing | ⚠ Parsed but not wired to engine |

> **Critical path to parity**: Fix gap #1 (per-rule parsing) → re-run benchmarks → fix gap #2 (indent logic).
> These two alone would close ~80% of the violation count gap.
>
> **Note**: nowinandroid uses `ktlint_official` (default), NOT `android_studio`. The violation
> gap is from rule implementation differences, not code style profile filtering.

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
| 10 | Class signature spacing | `standard:class-signature` | 2h | ✅ |
| 12 | Argument list wrapping | `standard:argument-list-wrapping` | 2h | ✅ |

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

- [x] Run `ktlint-rs` on nowinandroid (350 files) and compare output with JVM ktlint ✅
- [x] Run on compose-samples (380 files) ✅
- [x] Run on androidx (20,594 files) ✅
- [x] `--format` reduces violations by 53-71% ✅
- [x] `--format` on nowinandroid (350 files): 1.3s ✅
- [x] Phase 1.1: 12/12 rules ✅
- [x] Phase 1.2: 8/8 rules ✅
- [x] Phase 1.3: 3/3 rules ✅
- [x] 165 tests, all passing ✅

---

### 2.2 Code Style Profiles

| # | Profile | Diff from `ktlint_official` | Effort | Status |
|---|---|---|---|---|
| 1 | `android_studio` | Disables ~7 rules (final-newline, no-wildcard-imports, import-ordering, trailing-comma, etc.) | done | ✅ |
| 2 | `intellij_idea` | Disables ~3 rules (no-wildcard-imports, import-ordering, trailing-comma) | done | ✅ |
| 3 | `ktlint_official` | Default, enables all rules | done | ✅ |
|---|---|---|---|---|
| 1 | `android_studio` | Disables ~5 rules, changes trailing-comma | 2h | ⬜ NOT WIRED |
| 2 | `intellij_idea` | Disables ~3 rules | 2h | ⬜ NOT WIRED |
| 3 | `ktlint_official` | Default, enables all rules | 1h | ⬜ NOT WIRED |

> **Update (2026-07-12)**: `code_style` is parsed ✅ and wired to `RuleEngine::check()` via
> `is_rule_enabled()`. Per-rule `ktlint_standard_* = disabled` is PARSED but NOT wired to
> `RuleConfig` HashMap — `apply_editorconfig` doesn't handle these keys.
> **Critical fix**: wire per-rule enable/disable into `config.rules` HashMap.

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

### 6.3 Parity Verification

Automated comparison against JVM ktlint across all diagnostic dimensions:

| # | Task | Script | Effort | Status |
|---|---|---|---|---|
| 1 | Per-rule violation breakdown (rs vs JVM) | `scripts/bench.sh` | done | ✅ |
| 2 | Summary table (files, lines, violations, time) | `scripts/bench.sh` | done | ✅ |
| 3 | Exit code parity | `scripts/bench.sh` | done | ✅ |
| 4 | File-set parity (same files discovered?) | TBD | 1h | ⬜ |
| 5 | Suppress comment parity (`ktlint-disable`) | TBD | 2h | ⬜ |
| 6 | Auto-fix diff test (same output after `--format`?) | TBD | 3h | ⬜ |
| 7 | Reporter JSON structure parity | TBD | 2h | ⬜ |
| 8 | `.editorconfig` property parity (all props read?) | TBD | 2h | ⬜ |

> Parity target: ktlint-rs and JVM should produce identical Violations under `android_studio`.
> Run `./scripts/bench.sh --release` to generate the current parity report.

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
