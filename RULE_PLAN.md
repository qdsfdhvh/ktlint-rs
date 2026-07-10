# ktlint-rs Rule Implementation Plan

## Status: 12 compat / 1,038 JVM — gap = 18 rules

### Phase 1 — Highest Impact (1,026 violations)

| # | Rule | JVM viol | Effort | Notes |
|---|---|---|---|---|
| 1 | `multiline-expression-wrapping` | 741 | 4h | Our impl finds only 5. Need to study JVM's `MultilineExpressionWrappingRule.kt` |
| 2 | `no-empty-first-line-in-class-body` | 107 | 1h | We have stub, just enable + fix |
| 3 | `annotation` | 78 | 2h | Very complex (22.8K JVM rule). Multiple sub-checks |
| 4 | `when-entry-bracing` | 25 | 2h | JVM has `WhenEntryBracingTest.kt` |
| 5 | `blank-line-before-declaration` | 25 | 1h | We have stub |
| 6 | `blank-line-between-when-conditions` | 20 | 1h | We have stub |
| 7 | `indent` | 15 | 3h | Need auto-detect indent from project |
| 8 | `no-blank-line-in-list` | 12 | 1h | We have stub |

### Phase 2 — Medium Impact (19 violations)

| # | Rule | JVM viol | Effort |
|---|---|---|---|
| 9 | `wrapping` | 5 | 2h |
| 10 | `kdoc` | 5 | 1h |
| 11 | `argument-list-wrapping` | 5 | 2h |
| 12 | `parameter-list-wrapping` | 4 | 1h |
| 13 | `trailing-comma-on-call-site` | 3 | 1h |
| 14 | `no-consecutive-comments` | 3 | 0.5h |

### Phase 3 — Low Impact (4 violations)

| # | Rule | JVM viol | Effort |
|---|---|---|---|
| 15 | `no-blank-line-before-rbrace` | 1 | 0.5h |
| 16 | `spacing-between-declarations-with-comments` | 1 | 0.5h |
| 17 | `no-empty-file` | 1 | 0.5h |
| 18 | `filename` | 1 | 0.5h |

### JVM Reference Source

```
/tmp/ktlint/ktlint-ruleset-standard/src/main/kotlin/com/pinterest/ktlint/ruleset/standard/rules/
├── MultilineExpressionWrappingRule.kt  (741 violations)
├── AnnotationRule.kt                   (78 violations, 22.8K)
├── WhenEntryBracingTest.kt             (25 violations)
├── BlankLineBeforeDeclarationRule.kt   (25 violations)
├── BlankLineBetweenWhenConditions.kt   (20 violations)
├── NoBlankLineInListRule.kt           (12 violations)
├── WrappingRule.kt                     (5 violations)
├── KdocRule.kt                         (5 violations)
├── ArgumentListWrappingRule.kt        (5 violations)
├── ParameterListWrappingRule.kt       (4 violations)
├── TrailingCommaOnCallSiteRule.kt     (3 violations)
├── NoConsecutiveCommentsRule.kt       (3 violations)
├── FilenameRule.kt                     (1 violation)
└── ...
```

### Strategy

1. **Enable existing stubs**: 8 rules already have code in ktlint-rs, just disabled/no-op
2. **Implement from JVM source**: 6 rules need new implementation based on JVM Kotlin source
3. **Complex rules**: `annotation`, `wrapping` are large (20K+ lines of JVM source) — simplified versions will be implemented
4. **Test**: After each batch, run `KTLINT_COMPAT=1 target/release/ktlint tests/fixtures/nowinandroid` and compare with JVM

### Target

After all 18 rules: ktlint-rs compat mode should produce **1,000+ violations** matching JVM ktlint's 1,038.

### Estimated Total: ~25 hours
