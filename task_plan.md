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
| 11 | detekt L0 rules (empty-blocks 14, complexity 7) | 🟡 21/126 |
| 12 | Name resolution engine | ⬜ blocked |
| 13 | Type resolution bridge | ⬜ blocked |
| **8** | **Dual-engine architecture refactor** | ⬜ |
| **9** | **Unified config layer (.editorconfig + YAML unified)** | ⬜ |
| **10** | **detekt L0 rules (no type resolution, ~107 rules)** | ⬜ |
| **11** | **Name resolution engine (L1, ~50 rules)** | ⬜ |
| **12** | **Type resolution bridge (L2, ~51 rules)** | ⬜ |

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
| ktlint (formatting) | **78** | 105 |
| detekt (static analysis) | **21** (7 complexity + 14 empty-blocks) | 226 |
| **Total** | **99** | 331 |

**219 tests**, all passing. 7 reporter formats: plain, json, sarif, checkstyle, html, markdown, plain-summary.

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

1. **✅ Fix mod.rs duplicates & missing rules** — 4 rules were registered 10x each, causing massive false positives. Now 78 unique rules with no duplication.
2. **✅ Fix indent rule logic** — 6,948 → 369 violations. JVM-compatible brace tracking with `} else {` combo handling. Remaining gap (369 vs 15) from deeply-nested indent flakiness.
3. **✅ Tune new rules** — `blank-line-before-declaration`: 1,240 → 1. Now requires both current AND prev line to be declarations (matching JVM AST sibling check).
4. **🟡 Investigate rs-only rules** — `colon-spacing` fixed for annotations (196→35). Remaining: `no-unnecessary-parentheses-before-trailing-lambda` (389 vs 1), `function-naming` (324 vs 0), `no-empty-line-after-kdoc` (334 vs 0), `kdoc` (313 vs 5).
5. **✅ Fix `no-semicolons`** — 312→0 by tracking block comment state.

### Known Parity Gaps (nowinandroid, Jul 2026)

| # | Gap | rs | jvm | Status |
|---|---:|---:|---|
| 1 | ✅ per-rule disable | — | — | Done |
| 2 | ✅ blank-line-before-declaration | 1 | 25 | Under-flags |
| 3 | ✅ no-semicolons | 0 | 0 | Done |
| 4 | ✅ no-unnecessary-paren-lambda | 0 | 1 | Done (389→0) |
| 5 | ✅ colon-spacing | 35 | 0 | 196→35 |
| 6 | ✅ function-naming | 2 | 0 | 324→2 (CST + @Composable) |
| 7 | 🟡 **multiline-expression-wrapping** | 1,125 | 741 | +384 gap |
| 8 | 🟡 **indent** | 369 | 15 | +354 gap |
| 9 | 🟡 **kdoc** | 179 | 5 | +174 gap |
| 10 | 🟡 no-consecutive-comments | 100 | 3 | JVM more lenient |
| 11 | 🟡 annotation | 3 | 78 | **Under-flags** (misses same-line code) |
| 12 | 🟡 rs-only JVM=0 | ~600 | 0 | Experimental / ktlint-rs specific |
| 13 | 🟡 jvm-only RS=0 | 19 | — | 6 rules missing |
---

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
