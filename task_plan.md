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
| 5 | Advanced features (baselines, git hooks) | ⬜ |
| 6 | Testing & benchmarking | 🟡 |
| 7 | Distribution & docs | ⬜ |
| 8 | detekt static analysis rules (Phase 1: style) | ⬜ |
| 9 | detekt static analysis rules (Phase 2: complexity, naming, etc.) | ⬜ |

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

| Rule Set | Est. Rules | Overlap with ktlint-rs | Examples |
|---|---|---|---|
| `style` | ~45 | ~8 (no-wildcard-imports, no-unused-imports, max-line-length, filename, no-semi, etc.) | MagicNumber, UseCheckOrError, CollapsibleIfStatements, DataClassShouldBeImmutable, UnnecessaryAbstractClass |
| `complexity` | ~10 | 0 | CognitiveComplexMethod, LongMethod, LargeClass, NestedBlockDepth, CyclomaticComplexity |
| `exceptions` | ~12 | 0 | TooGenericExceptionCaught, SwallowedException, ThrowingExceptionsWithoutOrCause |
| `naming` | ~15 | ~3 (class/function/package naming) | BooleanPropertyNaming, MatchingDeclarationName, VariableNaming |
| `performance` | ~7 | 0 | ArrayPrimitive, SpreadOperator, UnnecessaryTemporaryInstantiation |
| `comments` | ~5 | ~1 (kdoc) | AbsentOrWrongFileLicense, EndOfSentenceFormat |
| `coroutines` | ~7 | 0 | GlobalCoroutineUsage, RedundantSuspendModifier, SuspendFunSwallowedCancellation |
| `empty-blocks` | ~4 | ~2 (no-empty-file, no-empty-class-body) | EmptyCatchBlock, EmptyFunctionBlock |
| **Total** | **~105** | **~14** | — |

### Key Differences: ktlint vs detekt

| Dimension | ktlint | detekt |
|---|---|---|
| **Scope** | Formatting (whitespace, imports, braces) | Static analysis (code smells, complexity, bugs) |
| **Input** | Text/CST only | Type resolution required for many rules |
| **Fixability** | Almost all auto-fixable | Most are advisory (manual refactor) |
| **Activation** | All rules enabled by default | Many rules opt-in ("Active by default: No") |
| **Complexity** | Regex + spacing analysis | AST traversal, control flow, type inference |

### Replacement Strategy

**Phase 8 (detekt style)** — Priority rules that overlap or bring high value:
- `MagicNumber` → already partially in ktlint-rs?
- `UseCheckOrError` / `UseRequire` — simple pattern match
- `CollapsibleIfStatements` — AST pattern
- `UnnecessaryAbstractClass` — AST analysis
- `NoSemicolons` — already covered by `no-semi`

**Phase 9 (remaining detekt categories)** — Harder rules requiring type resolution:
- Complexity rules need control flow analysis
- Exception rules need type hierarchy
- Coroutines rules need suspend-aware analysis

**Note**: Detekt 2.0+ supports Kotlin compiler type resolution. Replicating this in pure Rust without the Kotlin compiler is the major challenge — some detekt rules may be infeasible without it.
