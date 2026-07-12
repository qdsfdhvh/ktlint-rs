# Findings & Research Notes

## 2026-07-12: Violation Parity Root Cause Analysis

### .editorconfig Loading

- **Root cause**: `editorconfig` crate v1.x strips non-standard keys (`ktlint_code_style`, `ktlint_standard_*`, `ij_kotlin_*`).
- **Fix**: `parse_ktlint_properties()` manually reads the raw `.editorconfig` file.
- **Status**: Code style parsing works ✅. Per-rule disable (`ktlint_standard_<id> = disabled`) is parsed but NOT wired to `RuleConfig` HashMap.
- **Fix needed**: In `apply_editorconfig()`, properly handle `ktlint_standard_` keys → insert into `self.rules` map.

### nowinandroid Code Style

- **Finding**: nowinandroid uses `ktlint_official` (default), NOT `android_studio`.
- **Evidence**: `.editorconfig` has no `ktlint_code_style` key.
- **Implication**: All violation gaps are rule implementation differences, not profile filtering.

### Per-Rule Breakdown (nowinandroid)

- **rs-only (10 rules)**: colon-spacing, curly-spacing, modifier-order, op-spacing, spacing-around-angle-brackets, spacing-around-double-colon, spacing-around-keyword, string-template-indent, trailing-comma, when-expression-line-break
- **JVM-only (6 rules)**: filename, kdoc, no-empty-file, parameter-list-wrapping, trailing-comma-on-call-site, wrapping
- **Both but differ**: indent (6,948 vs 15), blank-line-before-declaration (1,240 vs 25), no-consecutive-comments (100 vs 3)

### Indent Rule

- ktlint-rs: simple "spaces % indent_size == 0" check. Flags every non-multiple-of-4 line.
- JVM: context-sensitive. Only flags inside blocks where previous line established a base indent.
- **Impact**: 6,948 false positives.

### New Rules Need Tuning

- `blank-line-before-declaration`: flags ANY `fun`/`val`/`class` keyword. JVM only checks top-level.
- `no-blank-line-in-list`: 39 vs 12. Line detection logic differs.
- `kdoc`: 0 vs 5. Inside-block detection not matching JVM's heuristic.

## 2026-07-11: Integration Test Infrastructure

- Submodules replaced with `scripts/setup-fixtures.sh` (shallow clone).
- Tests gracefully skip when fixtures absent.
- Bench script generates full parity report.

## 2026-07-10: Performance

- Rayon parallel processing: 10-27x faster than JVM.
- Apple M2, release build.
