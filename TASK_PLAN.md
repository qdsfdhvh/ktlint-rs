# ktlint-rs Project Plan

A pure-Rust Kotlin linter & formatter, drop-in compatible with [pinterest/ktlint](https://github.com/pinterest/ktlint).

**Goal**: Full JVM ktlint parity — same violations, same format output, 10-50x faster.

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
