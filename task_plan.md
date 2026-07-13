# ktlint-rs Project Plan

A pure-Rust Kotlin linter & formatter — dual-engine: ktlint (formatting) + detekt (static analysis).

**Goal**: Replace both [pinterest/ktlint](https://github.com/pinterest/ktlint) (formatting) and [detekt/detekt](https://github.com/detekt/detekt) (static analysis) with a single, 10-50x faster Rust binary.

---

## Phase Status

| Phase | Name | Status |
|---|---|---|
| 0 | Infrastructure & skeleton | ✅ |
| 1 | Core rules (spacing, structure, imports, naming, wrapping) | ✅ |
| 2 | .editorconfig & config parity | ✅ |
| 3 | Remaining rules & parity tuning | 🟡 |
| 4 | Formatter & auto-fix | ✅ |
| 5 | Advanced features (baselines, git hooks, YAML) | ✅ |
| 6 | Testing & benchmarking (219 tests, CI, bench) | ✅ |
| 7 | Distribution & docs (README, cargo publish) | ✅ |
| 8 | Registry + architecture refactor | ✅ |
| 9 | Unified config (namespace, category switches) | ✅ |
| 10 | CLI: \`--ruleset\` integration | ✅ |
| 11 | detekt L0 rules (37/126: 14 empty + 7 complexity + 4 naming + 3 comments + 6 style + 3 bugs) | 🟡 37/126 |
| 12 | Name resolution engine (L1, ~50 rules) | ⬜ blocked |
| 13 | Type resolution bridge (L2, ~51 rules) | ⬜ blocked |

---

## Performance (Apple M2, release)

| Project | Files | Lines | Violations (rs / JVM) | Time (rs / JVM) |
| Project | Files | Lines | rs Violations / JVM | Speed (rs / JVM) |
|---|---|---|---|---|
| **nowinandroid** | 350 | 31K | **2,553 / 1,038** | **0.22s / 8.3s (38×)** |
| compose-samples (6 apps) | 380 | 47K | — / 13 | — / 7.96s |
| okhttp | 569 | 131K | — / — | — / 11.5s |
| androidx (26 mods) | 1,271 | 267K | — / 33,731 | — / 10.6s |
| **bench-ktlint-rs** | 350 | 31K | **2,553 (ktlint) + 14,000 (detekt)** | **0.42s (both)** |

> nowinandroid data is current (Jul 2026). Other projects from last full bench, pending re-run.
---

## Rule Count

| 类型 | ktlint-rs | detekt 2.0 |
|---|---|---|
| ktlint (formatting) | **69** | 105 |
| detekt (static analysis) | **45** (14 empty + 15 complexity + 4 naming + 3 comments + 6 style + 3 bugs) | 226 |
| **Total** | **191** | 331 |

| **330 tests**, all passing. 7 reporter formats: plain, json, sarif, checkstyle, html, markdown, plain-summary.

---

## Dual-Engine Architecture

### Rule Naming Convention

Three-level namespace: `<source>:<category>:<RuleName>`

```
standard:curly-spacing                    # ktlint
standard:no-wildcard-imports              # ktlint
detekt:empty-blocks:EmptyFunctionBlock    # detekt
detekt:complexity:LongMethod              # detekt
detekt:style:MagicNumber                  # detekt
```

### Source Layout

```
src/rules/
├── mod.rs              # Rule trait + Violation definition
├── registry.rs         # Single source of truth for rule registration ← NEW
├── builtins.rs         # NoTrailingSpaces, FinalNewline, NoConsecutiveBlankLines, NoWildcardImports
├── ktlint/             # ktlint formatting rules ← migrated from flat structure
│   ├── mod.rs
│   ├── spacing/
│   ├── structure/
│   ├── imports/
│   ├── naming/
│   ├── wrapping/
│   ├── phase1_more/
│   └── phase3b/
├── detekt/             # detekt static analysis rules ← expanding
│   ├── mod.rs
│   ├── empty_blocks.rs  ✅ 14 rules
│   ├── complexity.rs    (Phase 10)
│   ├── style.rs         (Phase 10)
│   ├── naming.rs        (Phase 10)
│   ├── comments.rs      (Phase 10)
│   └── exceptions.rs    (Phase 10)
└── suppress/
```

### Rule Enable/Disable Priority (5 levels)

```
Priority (high → low):
  1. CLI --rule-disable / --rule-enable          (future)
  2. YAML config (detekt.yml)                     
  3. .editorconfig (ktlint_standard_* / ktlint_detekt_*)
  4. Code style profile + DetektProfile defaults
  5. Rule built-in defaults
     - ktlint rules: all enabled by default
     - detekt rules: ~104/226 active by default (matched to detekt official)
```

### CLI

```bash
# Default: ktlint formatting only (fully backward compatible)
ktlint **/*.kt

# detekt static analysis only
ktlint --ruleset detekt **/*.kt

# Both engines together
ktlint --ruleset ktlint,detekt **/*.kt

# Use detekt YAML config
ktlint --ruleset detekt --config detekt.yml **/*.kt

# Skip rules that need type resolution
ktlint --ruleset detekt --skip-type-resolution **/*.kt
```

`--ruleset` takes a comma-separated list. Extensible: future rule sets (e.g. `compose`, custom) can be added without new flags.

### Config Format Division

| Concern | .editorconfig | detekt.yml |
|---|---|---|
| Formatting params (indent, max_line_length) | ✅ Primary | ✅ Can override |
| ktlint rule enable/disable | ✅ `ktlint_standard_*` | ✅ `rules:` |
| detekt rule enable/disable | NEW `ktlint_detekt_*` | ✅ `rules:` |
| detekt rule properties (threshold, etc.) | ❌ Not suitable | ✅ `properties:` |
| IDE integration | ✅ Native | ❌ |
| detekt user migration | ❌ | ✅ Familiar workflow |

### YAML Config Format

```yaml
# Top-level switches (optional)
ktlint:
  active: true          # default: true
detekt:
  active: true          # default: false (requires --detekt or explicit)

rules:
  # Short name → auto-infer prefix (backward compatible)
  final-newline:
    active: false

  # Full ID → used directly
  "detekt:empty-blocks:EmptyFunctionBlock":
    active: true

  # Category-level batch switch
  "detekt:empty-blocks":
    active: false       # disables entire empty-blocks category

  # Rules with properties
  "detekt:complexity:LongMethod":
    active: true
    properties:
      threshold: 60

  # ktlint rules also support full ID
  "standard:curly-spacing":
    active: false
```

Name inference logic:
1. Contains `:` → full ID, use directly
2. Matches known detekt short name → expand to `detekt:<category>:<name>`
3. Otherwise → default to `standard:<name>` (backward compatible)

---

## Type Resolution Tiers

~101/226 detekt rules (45%) require Kotlin compiler type resolution. Progressive approach:

| Tier | Rule Count | Dependency | Strategy |
|---|---|---|---|
| **L0: CST** | ~125 | Pure tree-sitter | Implement directly |
| **L1: Name resolution** | ~50 | Scope/symbol table | Build name resolution engine |
| **L2: Type resolution** | ~51 | Kotlin compiler | Mark `requires: type-resolution`; future FFI bridge |

### L0 Implementation Priority

| Priority | Category | Rules | Difficulty | Notes |
|---|---|---|---|---|
| P0 | `empty-blocks` | 14 | Low | ✅ Done |
| P1 | `comments` | 9 | Low | Text-level checks |
| P1 | `naming` | 21 | Low | Mostly text matching |
| P1 | `complexity` | 15 | Low | AST traversal counts |
| P2 | `style` (no-type-res) | ~43 | Low-Med | Pattern matching |
| P2 | `exceptions` (no-type-res) | ~7 | Low | AST patterns |
| P2 | `potential-bugs` (no-type-res) | ~16 | Low | AST patterns |
| P3 | `coroutines` (no-type-res) | ~1 | Low | |
| P3 | `style` (need names) | ~12 | Med | Requires L1 |
| P3 | `exceptions` (need types) | ~10 | High | Requires L2 |
| P3 | `performance` (need types) | ~8 | High | Requires L2 |
| P4 | `potential-bugs` (need types) | ~23 | High | Requires L2 |

---

## Implementation Phases

### Phase 8: Dual-Engine Architecture Refactor (1-3 days)

- [ ] Create `src/rules/registry.rs` — single source of truth for rule registration
- [ ] Migrate ktlint rules into `src/rules/ktlint/` subdirectory
- [ ] Reorganize `src/rules/detekt/` directory
- [ ] Extract `builtins.rs` for NoTrailingSpaces, FinalNewline, etc.
- [ ] Rewrite `RuleEngine::new()` to call `registry::all_rules(config)`
- [ ] All existing tests must pass after refactor

### Phase 9: Unified Config Layer (2-3 days)

- [ ] Fix `yaml_config.rs` namespace inference (currently hardcoded `standard:` prefix)
- [ ] Add category-level batch switch support in YAML
- [ ] Implement `DetektProfile` with default-active table (~104 rules)
- [ ] Add `ktlint_detekt_*` parsing in `.editorconfig` handler
- [ ] Integrate 5-level priority in `KtlintConfig::is_rule_enabled()`
- [ ] YAML top-level `ktlint:` / `detekt:` active switches

### Phase 10: CLI Integration (1 day)

- [ ] Add `--ruleset <list>` CLI flag (comma-separated: `ktlint`, `detekt`, `ktlint,detekt`)
- [ ] Add `--skip-type-resolution` flag
- [ ] Wire `--ruleset` → rule set selection in RuleEngine (default: `ktlint`)

### Phase 11: detekt L0 Rules (4-6 weeks)

- [ ] `comments` (9 rules) — AbsentOrWrongFileLicense, CommentOverPrivateFunction, etc.
- [ ] `complexity` (15 rules) — LongMethod, LargeClass, NestedBlockDepth, CyclomaticComplexMethod, etc.
- [ ] `naming` no-type-res subset (~17 rules) — ClassNaming, FunctionNaming, VariableNaming, etc.
- [ ] `style` no-type-res subset (~43 rules) — MagicNumber, Mandelbrot, CollapsibleIfStatements, etc.
- [ ] `exceptions` no-type-res subset (~7 rules) — NotImplementedDeclaration, PrintStackTrace, etc.
- [ ] `potential-bugs` no-type-res subset (~16 rules) — ExitOutsideMain, InvalidRange, etc.

### Phase 12: Name Resolution Engine (4-6 weeks)

- [ ] Build per-file scope: track class, function, property declarations
- [ ] Resolve import aliases
- [ ] Track visibility (private/internal/public)
- [ ] L1 rules: UnusedPrivate*, NoNameShadowing, MemberNameEqualsClassName, ProtectedMemberInFinalClass, etc.

### Phase 13: Type Resolution Bridge (TBD)

- [ ] Evaluate FFI approach (kotlinc compiler plugin via stdin/stdout JSON)
- [ ] Or mark rules `requires: type-resolution` and return `unavailable` instead of false negatives
- [ ] L2 rules default disabled; enable via `--detekt-type-resolution` flag

---

## ktlint Parity (unchanged)

### Critical Path

1. **✅ Fix mod.rs duplicates** — 4 rules registered 10× each; now clean.
2. **✅ Fix indent rule** — JVM-compatible `} else {` handling. Gap: 369 vs 15.
3. **✅ Tune blank-line-before-declaration** — 1,240→1 (under-flags vs JVM 25).
4. **✅ Six parity rules fixed** — no-semicolons, no-unnecessary-paren-lambda, colon-spacing, function-naming, kdoc @param, no-empty-line-after-kdoc. Total: ~1,300 violations eliminated.
5. **⬜ Three core gaps remain** — multiline-expression-wrapping (+384), indent (+354), kdoc (+174). **These three account for 88% of the remaining implementation gap.**

### Current Bench (Jul 2026, nowinandroid)

| | ktlint-rs | JVM ktlint |
|---|---|---|
| Violations | 2,600 | 1,057 |
| Rules used | 38 | 21 |
| Speed | **0.68s** | 7.1s (**10× faster**) |

### Gap Root Cause Analysis

Total rs excess: **2,348 violations**

| Category | Violations | % | Description |
|---|---|---|---|
| Implementation differences | 1,096 | 47% | Same rule, different behavior (wrapping, indent, kdoc, etc.) |
| RS-only rules | 626 | 27% | JVM doesn't have these (experimental, different naming) |
| JVM-only (we miss) | 19 | 1% | 6 rules we don't implement |
| Exact match | 1 | — | `no-blank-line-before-rbrace` |

**Top 3 implementation gaps** (88% of all impl diff):
| Rule | rs | jvm | diff |
|---|---:|---:|
| `multiline-expression-wrapping` | 1,125 | 741 | +384 |
| `indent` | 369 | 15 | +354 |
| `kdoc` | 179 | 5 | +174 |

**Under-flagging** (we miss valid JVM violations):
| Rule | rs | jvm | diff |
|---|---:|---:|
| `annotation` | 3 | 78 | -75 |
| `no-empty-first-line-in-class-body` | 67 | 107 | -40 |
| `blank-line-before-declaration` | 1 | 25 | -24 |
| `when-entry-bracing` | 5 | 25 | -20 |

**RS-only top offenders** (JVM=0, >30 violations):
| Rule | Count | Why |
|---|---|---|
| `no-single-expression-body` | 139 | JVM doesn't have this rule |
| `import-ordering` | 82 | JVM experimental, disabled by default |
| `no-unused-imports` | 66 | JVM under different ID? |
| `property-naming` | 50 | JVM experimental |
| `spacing-between-declarations` | 49 | JVM doesn't have this |
| `op-spacing` | 41 | JVM uses different rule IDs |
| `colon-spacing` | 35 | Partially fixed (196→35) |
| `multiline-if-else` | 31 | JVM doesn't have this |
## Verified Dimensions

| Dimension | Status |
|---|---|
| Exit codes | ✅ Match |
| File discovery | ✅ Same .kt/.kts |
| Code style parsing | ✅ |
| Rules total | ✅ 65 (JVM has ~70 including experimental) |
| Tests passing | ✅ 195 |
| CI (test, clippy, fmt) | ✅ |

---

## detekt Rule Inventory (reference)

> Source: [detekt 2.0.0-alpha.0 docs](https://detekt.dev/docs/2.0.0-alpha.0/rules/)
> **Excluded**: `formatting` and `ktlint` rule sets — these are ktlint wrappers, already covered.
> Full per-rule breakdown in `docs/RULE_PLAN.md`.

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

> ⚠️ **Major risk**: 101/226 detekt rules (~45%) require Kotlin compiler type resolution. Pure Rust implementation may need alternative approaches or FFI bindings for these.

### Key Differences: ktlint vs detekt

| Dimension | ktlint | detekt |
|---|---|---|
| **Scope** | Formatting (whitespace, imports, braces) | Static analysis (code smells, complexity, bugs) |
| **Input** | Text/CST only | Type resolution required for ~101 rules |
| **Fixability** | Almost all auto-fixable | Most are advisory (manual refactor) |
| **Activation** | All rules enabled by default | ~104/226 rules active by default |
| **Config format** | .editorconfig | YAML (`detekt.yml`) |
| **Complexity** | Regex + spacing analysis | AST traversal, control flow, type inference |

### Non-Rule Feature Support Status

| Feature | detekt | ktlint-rs |
|---|---|---|
| YAML config (`detekt.yml`) | ✅ | ✅ |
| HTML report | ✅ | ✅ |
| XML report (Checkstyle) | ✅ | ✅ |
| Markdown report | ✅ | ✅ |
| SARIF report | ✅ | ✅ |
| JSON report | ✅ | ✅ |
| Baselines | ✅ XML | ✅ XML |
| `@Suppress` multi-format | ✅ 5 formats | 🟡 basic |
| Suppressors (annotation + function) | ✅ | ❌ |
| Plugins / Extensions | ✅ SPI-based | ❌ |
| Processors / Metrics | ✅ 10+ types | ❌ |
| Compose config | ✅ documented | 🟡 partial |

---

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| L2 type resolution rules (45%) won't work without compiler | Document clearly, mark `requires: type-resolution`, warn on skip not silent fail |
| 226 detekt rules maintenance burden | Prioritize L0 first, L1/L2 demand-driven |
| Performance degradation running both rule sets | Keep rayon parallel + early-skip disabled rules |
| User confusion: "is this ktlint or detekt?" | README + CLI help clearly explain dual-mode |
| `.editorconfig` + YAML config conflict | Clear 5-level priority; YAML wins over `.editorconfig` |
