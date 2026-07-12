# ktlint-rs Project Plan

A pure-Rust Kotlin linter & formatter, drop-in compatible with [pinterest/ktlint](https://github.com/pinterest/ktlint).

**Goal**: Replace both [pinterest/ktlint](https://github.com/pinterest/ktlint) (formatting) and [detekt/detekt](https://github.com/detekt/detekt) (static analysis) with a single, 10-50x faster Rust binary.

## Phase Status

| Phase | Name | Status |
|---|---|---|
| 0 | Infrastructure & skeleton | ✅ |
| 1 | Core rules (spacing, structure, imports, naming, wrapping) | ✅ |
| 2 | .editorconfig & config parity | ✅ |
| 3 | Remaining rules & parity tuning | 🟡 |
| 4 | Formatter & auto-fix | ✅ |
| 5 | Advanced features (baselines, git hooks, YAML config) | ⬜ |
| 6 | Testing & benchmarking | 🟡 |
| 7 | Distribution & docs | ⬜ |
| 8 | detekt static analysis rules (Phase 1: style, empty-blocks) | ⬜ |
| 9 | detekt static analysis rules (Phase 2: complexity, naming, bugs) | ⬜ |
| 10 | detekt non-rule features (reporters, suppressors, processors) | ⬜ |

## Performance (Apple M2, release)

| Project | Files | Lines | Violations (rs / JVM) | Time (rs / JVM) |
|---|---|---|---|---|
| nowinandroid | 350 | 31,021 | 9,901 / 1,038 | 0.26s / 6.71s |
| compose-samples (6 apps) | 380 | 46,586 | 10,752 / 13 | 0.30s / 7.96s |
| okhttp | 569 | 131,098 | 40,632 / 18 | 1.19s / 11.5s |
| androidx (26 modules) | 1,271 | 266,549 | 86,591 / 33,731 | 1.07s / 10.6s |

## Rule Count: 78 rules

- Spacing: 17 | Structure: 27 | Imports: 4 | Naming: 6 | Wrapping: 7 | Built-in: 3 | Phase/Final: 14
- Spacing: 17 | Structure: 28 | Imports: 4 | Naming: 6 | Wrapping: 7 | Built-in: 4
- New (2026-07-12): `blank-line-before-declaration`, `no-blank-line-in-list`, `kdoc` (enhanced)

## Critical Path to Parity

1. **✅ Fix mod.rs duplicates & missing rules** — 4 rules were registered 10x each, causing massive false positives. Now 78 unique rules with no duplication.
2. **Fix indent rule logic** — 6,948 vs 15 violations. JVM has context-sensitive indent. Closes ~50% gap.
3. **Tune new rules** — `blank-line-before-declaration` too aggressive. Needs to only check top-level declarations.
4. **Investigate rs-only rules** — `colon-spacing`, `op-spacing` etc. appear only in ktlint-rs. JVM might have these disabled by default?
## Known Parity Gaps (nowinandroid, ktlint_official)

| # | Gap | Impact | Root Cause |
|---|---|---|---|
| 1 | per-rule disable not wired | ~10 rules run incorrectly | `editorconfig` crate strips non-standard keys |
| 2 | indent rule | 6,948 vs 15 | Only "multiple of 4" check, no context |
| 3 | blank-line-before-declaration | 1,240 vs 25 | All decls vs top-level only |
| 4 | rs-only spacing rules | ~250 extra | JVM disables these by default? |
| 5 | JVM-only rules match 0 | filename, kdoc, wrapping | Implementation differs from JVM |

## Verified Dimensions

| Dimension | Status |
|---|---|
| Exit codes | ✅ Match |
| File discovery | ✅ Same .kt/.kts |
| Code style parsing | ✅ 
| Rules total | ✅ 65 (JVM has ~70 including experimental) |
| Tests passing | ✅ 185 |
| CI (test, clippy, fmt) | ✅ |

## detekt Comparison & Replacement Strategy

detekt has TWO layers of rules:
1. **Formatting rules** = ktlint wrapper (`detekt-rules-ktlint-wrapper`) — already covered by ktlint-rs
2. **Native static analysis rules** — ~100 rules across 8 categories, mostly disjoint from ktlint

### detekt Rule Sets (native, non-ktlint)

| Rule Set | Rules | Active by default | Type Res. Required | Overlap |
|---|---|---|---|---|
| `style` | 88 | ~25 | ~45 | ~5 |
| `potential-bugs` | 39 | ~25 | ~20 | 0 |
| `naming` | 21 | 5 | 1 | ~3 |
| `exceptions` | 17 | ~13 | ~10 | 0 |
| `complexity` | 15 | 11 | 3 | 0 |
| `empty-blocks` | 14 | 14 | 0 | ~2 |
| `performance` | 10 | 5 | 8 | 0 |
| `comments` | 9 | 0 | 4 | ~1 |
| `coroutines` | 8 | 5 | 7 | 0 |
| `libraries` | 3 | 1 | 3 | 0 |
| `ruleauthors` | 2 | 0 | 0 | 0 |
| **Total** | **226** | **~104** | **~101** | **~11** |

### Key Differences: ktlint vs detekt

| Dimension | ktlint | detekt |
|---|---|---|
| **Scope** | Formatting (whitespace, imports, braces) | Static analysis (code smells, complexity, bugs) |
| **Input** | Text/CST only | Type resolution required for ~101 rules |
| **Fixability** | Almost all auto-fixable | Most are advisory (manual refactor) |
| **Activation** | All rules enabled by default | ~104/226 rules active by default |
| **Config format** | .editorconfig | YAML (`detekt.yml`) |
| **Complexity** | Regex + spacing analysis | AST traversal, control flow, type inference |

### detekt Non-Rule Features to Support

| Feature | detekt | ktlint-rs | Phase |
|---|---|---|---|
| YAML config (`detekt.yml`) | ✅ | ❌ | 5 |
| HTML report (rich + metrics) | ✅ | ❌ | 10 |
| XML report (Checkstyle) | ✅ | ❌ | 10 |
| Markdown report | ✅ | ❌ | 10 |
| SARIF report | ✅ | ✅ | — |
| Baselines (XML) | ✅ | ❌ | 5 |
| `@Suppress` multi-format | ✅ 5 formats | 🟡 basic | 5 |
| Suppressors (annotation + function) | ✅ | ❌ | 10 |
| Plugins / Extensions | ✅ SPI-based | ❌ | — |
| Processors / Metrics | ✅ 10+ types | ❌ | 10 |
| Compose config | ✅ documented | 🟡 partial | 10 |

> ⚠️ **Major risk**: 101/226 detekt rules (~45%) require Kotlin compiler type resolution. Pure Rust implementation may need alternative approaches or FFI bindings for these.
