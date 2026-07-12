# ktlint-rs Rule Plan

> **Target**: 100% parity with JVM ktlint (formatting rules) + detekt (static analysis rules).
> **Coverage**: 61/105 JVM ktlint rules ✅ | 44 missing ❌ | ~105 detekt rules (Phase 8-9).

---

## Part 1: ktlint Standard Rules (JVM)

Status legend: ✅ implemented | 🟡 partial / needs tuning | ❌ missing

### Spacing Rules

| Rule ID | Status | Notes |
|---|---|---|
| `annotation-spacing` | ✅ | |
| `block-comment-initial-star-alignment` | ✅ | `block-comment-star` in rs |
| `class-signature` | ✅ | `class-signature-spacing` in rs |
| `comment-spacing` | ✅ | |
| `fun-keyword-spacing` | ✅ | `function-name-paren-spacing` in rs |
| `function-return-type-spacing` | ✅ | |
| `function-signature` | ✅ | `function-signature-spacing` in rs |
| `function-start-of-body-spacing` | ✅ | |
| `modifier-list-spacing` | ❌ | |
| `modifier-order` | ✅ | |
| `nullable-type-spacing` | ❌ | |
| `parameter-list-spacing` | ✅ | |
| `spacing-around-angle-brackets` | ✅ | |
| `spacing-around-colon` | ✅ | `colon-spacing` in rs |
| `spacing-around-comma` | ✅ | `comma-spacing` in rs |
| `spacing-around-curly` | ✅ | `curly-spacing` in rs |
| `spacing-around-dot` | ❌ | |
| `spacing-around-double-colon` | ✅ | `double-colon-spacing` in rs |
| `spacing-around-keyword` | ✅ | |
| `spacing-around-operators` | ✅ | `operator-spacing` in rs |
| `spacing-around-parens` | ✅ | `paren-spacing` in rs |
| `spacing-around-range-operator` | ✅ | `range-operator-spacing` in rs |
| `spacing-around-square-brackets` | ❌ | |
| `spacing-around-unary-operator` | ❌ | |
| `spacing-between-declarations-with-annotations` | ❌ | |
| `spacing-between-declarations-with-comments` | ✅ | |
| `spacing-between-function-name-and-opening-parenthesis` | ✅ | |
| `then-spacing` | ❌ | |
| `try-catch-finally-spacing` | ❌ | |
| `type-argument-list-spacing` | ✅ | |
| `type-parameter-list-spacing` | ❌ | |
| `function-type-modifier-spacing` | ❌ | |
| `function-type-reference-spacing` | ❌ | |
| `package-import-spacing` | ❌ | |

### Wrapping Rules

| Rule ID | Status | Notes |
|---|---|---|
| `argument-list-wrapping` | ✅ | |
| `binary-expression-wrapping` | ❌ | |
| `call-expression-wrapping` | ❌ | |
| `chain-method-continuation` | ❌ | |
| `chain-wrapping` | ✅ | |
| `comment-wrapping` | ❌ | |
| `context-receiver-list-wrapping` | ❌ | |
| `context-receiver-wrapping` | ❌ | |
| `enum-wrapping` | ✅ | |
| `expression-operand-wrapping` | ❌ | |
| `if-else-wrapping` | ❌ | |
| `kdoc-wrapping` | ❌ | |
| `multi-line-if-else` | ✅ | `multiline-if-else` in rs |
| `multiline-expression-wrapping` | ✅ | |
| `multiline-loop` | ❌ | |
| `parameter-list-wrapping` | ❌ | |
| `parameter-wrapping` | ❌ | |
| `property-wrapping` | ❌ | |
| `statement-wrapping` | ❌ | |
| `string-template-indent` | ✅ | |
| `wrapping` | ✅ | `general-wrapping` in rs |

### Structure / Blank-line Rules

| Rule ID | Status | Notes |
|---|---|---|
| `blank-line-before-declaration` | 🟡 | Too aggressive — needs top-level only |
| `blank-line-before-file-annotation` | ❌ | |
| `blank-line-before-imports` | ❌ | |
| `blank-line-before-package` | ❌ | |
| `blank-line-between-when-conditions` | ✅ | |
| `final-newline` | ✅ | |
| `indentation` | 🟡 | 6,948 vs 15 — needs context-aware logic |
| `max-line-length` | ✅ | |
| `no-blank-line-before-rbrace` | ✅ | |
| `no-blank-line-in-list` | ✅ | |
| `no-blank-lines-in-chained-method-calls` | ❌ | |
| `no-consecutive-blank-lines` | ✅ | |
| `no-consecutive-comments` | ❌ | |
| `no-empty-class-body` | ✅ | |
| `no-empty-file` | ✅ | |
| `no-empty-first-line-in-class-body` | ✅ | |
| `no-empty-first-line-in-method-block` | ✅ | `no-leading-empty-lines-in-method` in rs |
| `no-line-break-after-else` | ❌ | |
| `no-line-break-before-assignment` | ❌ | |
| `no-multiple-spaces` | ✅ | `no-multi-spaces` in rs |
| `no-semicolons` | ✅ | |
| `no-single-line-block-comment` | ❌ | |
| `no-trailing-spaces` | ✅ | |
| `no-unit-return` | ❌ | |
| `trailing-comma-on-call-site` | ✅ | |
| `trailing-comma-on-declaration-site` | ✅ | |
| `unnecessary-parentheses-before-trailing-lambda` | ✅ | |

### Import Rules

| Rule ID | Status | Notes |
|---|---|---|
| `import-ordering` | ✅ | |
| `no-unused-imports` | ✅ | |
| `no-wildcard-imports` | ✅ | |

### Naming Rules

| Rule ID | Status | Notes |
|---|---|---|
| `backing-property-naming` | ✅ | |
| `class-naming` | ✅ | |
| `enum-entry-name-case` | ✅ | `enum-entry` in rs |
| `filename` | 🟡 | JVM detects 0 matches (implementation differs) |
| `function-naming` | ✅ | |
| `package-name` | ✅ | |
| `property-naming` | ✅ | |

### KDoc Rules

| Rule ID | Status | Notes |
|---|---|---|
| `kdoc` | ✅ | `kdoc-formatting` in rs |
| `kdoc-wrapping` | ❌ | |

### Expression / Brace Rules

| Rule ID | Status | Notes |
|---|---|---|
| `function-expression-body` | ✅ | |
| `function-literal` | ❌ | |
| `if-else-bracing` | ❌ | |
| `lambda-return` | ❌ | |
| `mixed-condition-operators` | ❌ | |
| `string-template` | ❌ | |
| `when-entry-bracing` | ✅ | |

### Argument / Type Comment Rules

| Rule ID | Status | Notes |
|---|---|---|
| `type-argument-comment` | ❌ | |
| `type-parameter-comment` | ❌ | |
| `value-argument-comment` | ❌ | |
| `value-parameter-comment` | ❌ | |

---

## Part 2: ktlint-rs-only Rules (no direct JVM equivalent)

These exist in rs but not as separate JVM rules (subset, combined, or internal):

| Rule ID | Notes |
|---|---|
| `ij-trailing-comma` | IntelliJ-specific — keep for IDE compat |
| `kdoc-no-empty-first-line` | Subset of JVM `kdoc` |
| `kdoc-no-trailing-space` | Subset of JVM `kdoc` |
| `ktlint-annotation` | ktlint internal — keep |
| `ktlint-wrapping` | ktlint internal — keep |
| `ktlint-no-consecutive-comments` | ktlint internal — keep |
| `lambda-paren` | May overlap with `function-literal` |
| `no-blank-after-kdoc` | Subset of JVM `kdoc` |
| `no-blank-before-list-close` | Subset of `parameter-list-spacing` |
| `no-empty-file-body` | No JVM equivalent |
| `no-single-expression-body` | No JVM equivalent |
| `no-trailing-spaces-in-string` | No JVM equivalent |
| `no-wildcard-imports-either` | rs-only variant |
| `spacing-between-declarations` | rs-only — JVM has annotated/commented variants |
| `trailing-comma` | Generic — JVM has site-specific variants |
| `trailing-spaces-in-comment` | Might overlap with `comment-spacing` |
| `try-catch-finally-wrapping` | rs-only — JVM has `try-catch-finally-spacing` |
| `when-expression-line-break` | rs-only variant |

---

## Part 3: detekt Static Analysis Rules (Phase 8-9)

> detekt wraps ktlint via `detekt-rules-ktlint-wrapper` for formatting.
> Below are detekt's NATIVE rules (code quality, not formatting).

### style (~45 rules)

| Rule ID | Priority | Notes |
|---|---|---|
| `MagicNumber` | High | CST-based (detect literal numbers) |
| `UseCheckOrError` | High | `throw IllegalStateException` → `error()` |
| `UseRequire` | High | `throw IllegalArgumentException` → `require()` |
| `CollapsibleIfStatements` | Medium | Nested if → combined condition |
| `DataClassShouldBeImmutable` | Medium | Detect `var` in data classes |
| `UnnecessaryAbstractClass` | Medium | No abstract members → concrete |
| `CanBeNonNullable` | Medium | Redundant nullability |
| `RedundantExplicitType` | Medium | Needs type resolution ❗ |
| `UnnecessaryFullyQualifiedName` | Low | Needs type resolution ❗ |
| `NoTabs` | Low | Already partly covered by spacing rules |
| `ForbiddenComment` | Low | Pattern match |
| `LoopWithTooManyJumpStatements` | Low | Control flow analysis |
| `MaxLineLength` | — | Already ✅ in ktlint-rs |
| `NoWildcardImports` | — | Already ✅ in ktlint-rs |
| `NoUnusedImports` | — | Already ✅ in ktlint-rs |
| `NoSemicolons` | — | Already ✅ in ktlint-rs |
| `Filename` | — | Already ✅ in ktlint-rs |
| _...remaining ~30 style rules_ | TBD | Needs full rule list extraction |

### complexity (~10 rules)

| Rule ID | Priority | Notes |
|---|---|---|
| `CognitiveComplexMethod` | Medium | Needs control flow graph |
| `LongMethod` | Medium | LOC count |
| `LargeClass` | Medium | Member count |
| `NestedBlockDepth` | Medium | AST depth count |
| `CyclomaticComplexity` | Medium | Branch count |
| `LongParameterList` | Medium | Parameter count |
| `ComplexCondition` | Low | Boolean expression complexity |
| `StringLiteralDuplication` | Low | String comparison |
| `TooManyFunctions` | Low | Function count per file/class |
| `ComplexInterface` | Low | Method count |

### exceptions (~12 rules)

| Rule ID | Priority | Notes |
|---|---|---|
| `TooGenericExceptionCaught` | High | Catch `Exception` / `Throwable` |
| `SwallowedException` | Medium | Empty catch with no logging |
| `ThrowingExceptionsWithoutOrCause` | Medium | Re-throw without wrapping |
| `InstanceOfCheckForException` | Low | |
| `NotImplementedDeclaration` | Low | `TODO()` body |
| _...remaining ~7 rules_ | TBD | |

### naming (~15 rules)

| Rule ID | Priority | Notes |
|---|---|---|
| `BooleanPropertyNaming` | Medium | `is`/`has` prefix |
| `MatchingDeclarationName` | Medium | Class name ≠ file name |
| `VariableNaming` | Medium | Pattern-based |
| `FunctionParameterNaming` | Low | |
| `ObjectPropertyNaming` | Low | |
| `TopLevelPropertyNaming` | Low | |
| _...remaining ~9 rules_ | TBD | |

### performance (~7 rules)

| Rule ID | Priority | Notes |
|---|---|---|
| `ArrayPrimitive` | Medium | `Array<Int>` → `IntArray` |
| `SpreadOperator` | Medium | `*array` performance |
| `UnnecessaryTemporaryInstantiation` | Low | |
| _...remaining ~4 rules_ | TBD | |

### comments (~5 rules)

| Rule ID | Priority | Notes |
|---|---|---|
| `AbsentOrWrongFileLicense` | Low | Pattern match |
| `EndOfSentenceFormat` | Low | KDoc sentence ending |
| _...remaining ~3 rules_ | TBD | |

### coroutines (~7 rules)

| Rule ID | Priority | Notes |
|---|---|---|
| `GlobalCoroutineUsage` | High | `GlobalScope.*` detection |
| `RedundantSuspendModifier` | Medium | No suspend calls in body |
| `SuspendFunSwallowedCancellation` | Medium | |
| _...remaining ~4 rules_ | TBD | |

### empty-blocks (~4 rules)

| Rule ID | Priority | Notes |
|---|---|---|
| `EmptyCatchBlock` | High | CST-based |
| `EmptyFunctionBlock` | Medium | CST-based |
| `EmptyIfBlock` | Medium | CST-based |
| `EmptyElseBlock` | Medium | CST-based |

---

## Part 4: Implementation Priority Path

### Phase 3 (current) — ktlint parity tuning
1. 🟡 `indentation` — fix context-aware indent (6,948 → ~15)
2. 🟡 `blank-line-before-declaration` — top-level only
3. ❌ `spacing-between-declarations-with-annotations`
4. ❌ `no-consecutive-comments`

### Phase 5 (advanced features)
- Baselines
- Git hooks

### Phase 6 (testing)
- Parity regression tests
- Benchmark suite

### Phase 8 — High-value detekt rules (no type resolution needed)
1. `MagicNumber`
2. `UseCheckOrError` / `UseRequire`
3. `CollapsibleIfStatements`
4. `EmptyCatchBlock` / `EmptyFunctionBlock` / `EmptyIfBlock`
5. `TooGenericExceptionCaught`
6. `GlobalCoroutineUsage`
7. `DataClassShouldBeImmutable`

### Phase 9 — Harder detekt rules (type resolution / control flow)
- `RedundantExplicitType`, `UnnecessaryFullyQualifiedName`
- `CognitiveComplexMethod`, `CyclomaticComplexity`
- `BooleanPropertyNaming`, `VariableNaming`

> ⚠️ **Major risk**: Many detekt rules require Kotlin compiler type resolution (Detekt 2.0+). Pure Rust implementation may need alternative approaches or FFI bindings.

---

## Summary

| Layer | Total | ✅ Done | 🟡 Needs Tuning | ❌ Missing |
|---|---|---|---|---|
| ktlint standard (JVM) | 105 | 61 | 3 | 41 |
| ktlint-rs-only | 18 | 18 | 0 | 0 |
| detekt style | ~45 | ~5 | 0 | ~40 |
| detekt other | ~60 | 0 | 0 | ~60 |
| **Total** | **~228** | **84** | **3** | **~141** |
