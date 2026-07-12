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

## 2026-07-12: detekt Non-Rule Feature Surface

Analysis of detekt 2.0.0-alpha.0 non-rule features that ktlint-rs would need to replace.

### Configuration (YAML)
- detekt uses YAML config (`detekt.yml`), NOT .editorconfig
- Supports: rule set/rule properties, path filters (includes/excludes per rule), console reports, output reports, processors (metrics)
- `--build-upon-default-config` flag merges with defaults
- ktlint-rs gap: .editorconfig only, no YAML

### Reporting
- 4 output formats: **HTML** (rich with metrics), **XML** (Checkstyle), **Markdown**, **SARIF**
- Console reports: ProjectStatistics, Complexity, Notification, Findings, FileBasedFindings, LiteFindings
- ktlint-rs: SARIF ✅, JSON ✅, plain/lite ✅; missing HTML, XML, Markdown

### Suppression
- `@Suppress("RuleId")` / `@SuppressWarnings("RuleId")`
- Multiple ID formats: `rule`, `ruleset:rule`, `detekt:ruleset:rule`, `all`
- ktlint-rs: basic `@Suppress` ✅, but detekt's multi-format resolution is richer

### Baselines
- XML format (`baseline.xml`), CLI `--baseline` + `--create-baseline`
- Suppresses known issues for legacy projects
- ktlint-rs: ❌ (planned Phase 5)

### Suppressors (global suppression filters)
- **Annotation Suppressor**: `ignoreAnnotated: ['Preview']` — suppresses issues under annotated code
- **Function Suppressor**: `ignoreFunction: ['java.time.LocalDate.now']` — suppresses issues in specific function definitions
- ktlint-rs: ❌

### Compose Support
- Configuration recommendations for @Composable functions (naming patterns, TooManyFunctions exclusions)
- Not a separate rule set — just config tuning for built-in rules
- ktlint-rs: partially covered if the same rules exist

### Extensions / Plugins
- Plugin system: `--plugins /path/to/jar` (CLI) or `detektPlugins()` (Gradle)
- Custom `RuleSetProvider` implementations via SPI
- Testing framework: `detekt-test` with `Rule.lint()` helpers
- ktlint-rs: ❌ no plugin system

### Processors (Metrics)
- KtFileCount, PackageCount, ClassCount, FunctionCount, PropertyCount
- ProjectComplexity, ProjectCognitiveComplexity
- ProjectLLOC/CLOC/LOC/SLOC (line counts)
- ktlint-rs: ❌

### Feature Gap Summary

| Feature | detekt | ktlint-rs |
|---|---|---|
| Config format | YAML | .editorconfig + CLI args |
| Path filters | ✅ includes/excludes | ❌ (just glob patterns) |
| HTML report | ✅ rich + metrics | ❌ |
| XML report | ✅ Checkstyle | ❌ |
| Markdown report | ✅ | ❌ |
| Baselines | ✅ XML | ❌ (Phase 5 planned) |
| `@Suppress` multi-format | ✅ 5 formats | 🟡 basic only |
| Suppressors | ✅ annotation + function | ❌ |
| Plugins | ✅ SPI-based | ❌ |
| Processors/metrics | ✅ 10+ metrics | ❌ |
| Compose config | ✅ documented | 🟡 partial |
