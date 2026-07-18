# ktlint-rs Rule Plan

> **Target**: 100% parity with JVM ktlint (formatting rules) + detekt (static analysis rules).
> **Coverage**: 69/105 JVM ktlint rules ✅ | 154 detekt rules (L0+L1) ✅ | L2 ~51 rules (skip mechanism ready) ✅

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

## Part 3: detekt Native Rules — Full Inventory (226 rules)

> Source: [detekt 2.0.0-alpha.0 docs](https://detekt.dev/docs/2.0.0-alpha.0/rules/)
> **Excluded**: `formatting` and `ktlint` rule sets — these are ktlint wrappers, already covered by Part 1.
> **Overlap**: ~8 rules already ✅ in ktlint-rs (MaxLineLength, NoWildcardImports, NoSemicolons, Filename, etc.)

### comments (9 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `AbsentOrWrongFileLicense` | No | — |
| `CommentOverPrivateFunction` | No | — |
| `DeprecatedBlockTag` | No | — |
| `EndOfSentenceFormat` | No | — |
| `KDocReferencesNonPublicProperty` | No | ❗ |
| `OutdatedDocumentation` | No | ❗ |
| `UndocumentedPublicClass` | No | ❗ |
| `UndocumentedPublicFunction` | No | ❗ |
| `UndocumentedPublicProperty` | No | ❗ |

### complexity (15 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `CognitiveComplexMethod` | Yes | — |
| `ComplexCondition` | Yes | — |
| `ComplexInterface` | Yes | — |
| `CyclomaticComplexMethod` | Yes | — |
| `LabeledExpression` | No | — |
| `LargeClass` | Yes | — |
| `LongMethod` | Yes | — |
| `LongParameterList` | Yes | ❗ |
| `MethodOverloading` | Yes | — |
| `NamedArguments` | No | ❗ |
| `NestedBlockDepth` | Yes | — |
| `NestedScopeFunctions` | No | — |
| `ReplaceSafeCallChainWithRun` | No | — |
| `StringLiteralDuplication` | Yes | — |
| `TooManyFunctions` | Yes | ❗ |

### coroutines (8 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `CoroutineLaunchedInTestWithoutRunTest` | Yes | ❗ |
| `GlobalCoroutineUsage` | Yes | ❗ |
| `InjectDispatcher` | No | — |
| `RedundantSuspendModifier` | Yes | ❗ |
| `SleepInsteadOfDelay` | Yes | ❗ |
| `SuspendFunSwallowedCancellation` | Yes | ❗ |
| `SuspendFunWithCoroutineScopeReceiver` | No | ❗ |
| `SuspendFunWithFlowReturnType` | No | ❗ |

### empty-blocks (14 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `EmptyCatchBlock` | Yes | — |
| `EmptyClassBlock` | Yes | — |
| `EmptyDoWhileBlock` | Yes | — |
| `EmptyElseBlock` | Yes | — |
| `EmptyFinallyBlock` | Yes | — |
| `EmptyForBlock` | Yes | — |
| `EmptyFunctionBlock` | Yes | — |
| `EmptyIfBlock` | Yes | — |
| `EmptyInitBlock` | Yes | — |
| `EmptyKtFile` | Yes | — |
| `EmptySecondaryConstructor` | Yes | — |
| `EmptyTryBlock` | Yes | — |
| `EmptyWhenBlock` | Yes | — |
| `EmptyWhileBlock` | Yes | — |

### exceptions (17 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `ErrorUsageWithThrowable` | Yes | — |
| `ExceptionRaisedInCurrentContext` | Yes | ❗ |
| `ExceptionRaisedInUnexpectedLocation` | No | ❗ |
| `InstanceOfCheckForException` | Yes | ❗ |
| `NotImplementedDeclaration` | Yes | — |
| `ObjectExtendsThrowable` | Yes | — |
| `PrintStackTrace` | Yes | — |
| `RethrowCaughtException` | Yes | ❗ |
| `ReturnFromFinally` | Yes | — |
| `SwallowedException` | Yes | ❗ |
| `ThrowingExceptionFromFinally` | Yes | — |
| `ThrowingExceptionInMain` | No | — |
| `ThrowingExceptionsWithoutOrCause` | Yes | ❗ |
| `ThrowingNewInstanceOfSameException` | Yes | ❗ |
| `TooGenericExceptionCaught` | Yes | ❗ |
| `TooGenericExceptionThrown` | Yes | ❗ |
| `UnusedProcessNextRuntimeException` | Yes | ❗ |

### libraries (3 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `ForbiddenPublicDataClass` | No | ❗ |
| `LibraryCodeMustSpecifyReturnType` | No | ❗ |
| `LibraryEntitiesShouldNotBePublic` | Yes | ❗ |

### naming (21 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `BooleanPropertyNaming` | No | — |
| `ClassNaming` | Yes | — |
| `ConstructorParameterNaming` | No | — |
| `EnumNaming` | Yes | — |
| `ForbiddenClassName` | No | — |
| `FunctionMaxLength` | No | — |
| `FunctionMinLength` | No | — |
| `FunctionNaming` | Yes | — |
| `FunctionParameterNaming` | No | — |
| `InvalidPackageDeclaration` | Yes | — |
| `LambdaParameterNaming` | No | — |
| `MatchingDeclarationName` | No | — |
| `MemberNameEqualsClassName` | No | ❗ |
| `NoNameShadowing` | Yes | ❗ |
| `NonBooleanPropertyPrefixedWithIs` | No | — |
| `ObjectPropertyNaming` | No | — |
| `PackageNaming` | Yes | — |
| `TopLevelPropertyNaming` | No | — |
| `VariableMaxLength` | No | — |
| `VariableMinLength` | No | — |
| `VariableNaming` | No | — |

### performance (10 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `ArrayPrimitive` | Yes | ❗ |
| `CouldBeSequence` | No | ❗ |
| `DoubleMutabilityForCollection` | No | — |
| `ForEachOnRange` | Yes | ❗ |
| `HashtableSize` | No | ❗ |
| `SpreadOperator` | Yes | ❗ |
| `UnnecessaryPartOfBinaryExpression` | Yes | — |
| `UnnecessaryTemporaryInstantiation` | Yes | ❗ |
| `UnnecessaryTypeCasting` | No | ❗ |
| `UnnecessaryToString` | No | ❗ |

### potential-bugs (39 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `AvoidDirectByteBuffer` | Yes | ❗ |
| `AvoidReferentialEquality` | No | ❗ |
| `AvoidUsingVolatile` | No | — |
| `CastDueToProgressionResolution` | Yes | ❗ |
| `CastNullableToNonNullableType` | Yes | ❗ |
| `CastToNotNullableType` | Yes | ❗ |
| `Deprecation` | Yes | ❗ |
| `DontDowncastCollectionTypes` | Yes | ❗ |
| `ElseCaseInsteadOfExhaustiveWhen` | No | — |
| `EqualsAlwaysReturnsTrueOrFalse` | Yes | ❗ |
| `EqualsWithHashCodeExist` | Yes | ❗ |
| `ExitOutsideMain` | Yes | — |
| `ExplicitGarbageCollectionCall` | Yes | — |
| `ExpressionBodySyntax` | No | ❗ |
| `HasPlatformType` | Yes | ❗ |
| `IgnoredReturnValue` | Yes | ❗ |
| `ImplicitDefaultLocale` | Yes | ❗ |
| `ImplicitUnitReturnType` | No | ❗ |
| `InvalidRange` | Yes | — |
| `IteratorHasNextCallsNextMethod` | Yes | — |
| `IteratorNotThrowingNoSuchElementException` | Yes | — |
| `LateinitUsage` | No | — |
| `MapGetWithNotNullAssertionOperator` | Yes | ❗ |
| `MapOperationGrouping` | No | ❗ |
| `MissingPackageDeclaration` | Yes | — |
| `NullCheckOnMutableProperty` | Yes | — |
| `NullableToStringCall` | Yes | ❗ |
| `PropertyUsedBeforeDeclaration` | No | — |
| `RedundantElseInWhen` | Yes | — |
| `UnconditionalJumpStatementInLoop` | No | — |
| `UnreachableCatchBlock` | Yes | ❗ |
| `UnreachableCode` | Yes | — |
| `UnsafeCallOnNullableType` | Yes | — |
| `UnsafeCast` | Yes | — |
| `UnusedUnaryOperator` | Yes | ❗ |
| `UselessPostfixExpression` | Yes | ❗ |
| `WrongEqualsTypeParameter` | Yes | ❗ |

### ruleauthors (2 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `ForbiddenSuppress` | No | — |
| `UseEntityAtCurrentDirectory` | No | — |

### style (88 rules)

| Rule ID | Active | Type Resolution |
|---|---|---|
| `AbstractClassCanBeConcreteClass` | Yes | ❗ |
| `AlsoCouldBeApply` | No | — |
| `BracesOnIfStatements` | No | — |
| `BracesOnWhenStatements` | No | — |
| `CanBeNonNullable` | Yes | ❗ |
| `CascadingCallWrapping` | No | — |
| `ClassOrdering` | No | ❗ |
| `CollapsibleIfStatements` | Yes | — |
| `ConstructorParameterOrdering` | No | — |
| `DataClassContainsFunctions` | Yes | ❗ |
| `DataClassShouldBeImmutable` | No | ❗ |
| `DestructuringDeclarationWithTooManyEntries` | Yes | — |
| `DoubleNegativeLambda` | No | ❗ |
| `EqualsNullCall` | No | — |
| `EqualsOnSignatureLine` | No | — |
| `ExplicitCollectionElementAccessMethod` | No | ❗ |
| `ExplicitItLambdaParameter` | No | — |
| `ExpressionBodySyntax` | No | ❗ |
| `ForbiddenAnnotation` | No | ❗ |
| `ForbiddenComment` | No | — |
| `ForbiddenImport` | No | — |
| `ForbiddenMethodCall` | No | ❗ |
| `ForbiddenSuppress` | No | — |
| `ForbiddenVoid` | Yes | — |
| `FunctionOnlyReturningConstant` | Yes | ❗ |
| `LoopWithTooManyJumpStatements` | Yes | — |
| `MagicNumber` | Yes | ❗ |
| `Mandelbrot` | No | — |
| `MaxChainedCallsOnSameLine` | No | — |
| `MayBeConst` | No | ❗ |
| `ModifierOrder` | No | ❗ |
| `MultilineLambdaItParameter` | No | — |
| `MultilineRawStringIndentation` | No | — |
| `NewLineAtEndOfFile` | Yes | — |
| `NoTabs` | No | — |
| `NullableBooleanProperty` | No | — |
| `ObjectLiteralToLambda` | No | ❗ |
| `OptionalAbstractKeyword` | No | — |
| `OptionalUnit` | No | — |
| `OptionalWhenBraces` | No | — |
| `PreferToOverPairSyntax` | No | ❗ |
| `ProtectedMemberInFinalClass` | Yes | ❗ |
| `RedundantVisibilityModifierRule` | No | ❗ |
| `ReturnCount` | No | — |
| `SerialVersionUIDInSerializableClass` | No | ❗ |
| `SpacingBetweenPackageAndImports` | No | — |
| `ThrowsCount` | No | — |
| `TrailingWhitespace` | No | — |
| `TrimMultilineRawString` | No | — |
| `UnderscoresInNumericLiterals` | No | — |
| `UnnecessaryAnnotationUseSiteTarget` | No | — |
| `UnnecessaryApply` | No | ❗ |
| `UnnecessaryBackticks` | No | — |
| `UnnecessaryBracesInTrailingLambda` | No | — |
| `UnnecessaryFilter` | No | ❗ |
| `UnnecessaryInnerClass` | No | ❗ |
| `UnnecessaryLet` | No | ❗ |
| `UnnecessaryMixIn` | No | ❗ |
| `UnnecessaryStringReplaceAll` | No | ❗ |
| `UntilInsteadOfRangeTo` | No | ❗ |
| `UnusedParameter` | No | ❗ |
| `UnusedPrivateClass` | Yes | ❗ |
| `UnusedPrivateFunction` | No | ❗ |
| `UnusedPrivateProperty` | Yes | ❗ |
| `UseAnyOrNoneInsteadOfFind` | No | ❗ |
| `UseArrayLiteralsInAnnotations` | Yes | — |
| `UseCheckNotNull` | No | ❗ |
| `UseCheckOrError` | Yes | ❗ |
| `UseChunkedInsteadOfWindowedWithSameParam` | No | ❗ |
| `UseDataClass` | No | ❗ |
| `UseEmptyCounterpart` | No | — |
| `UseEmptyRequestBody` | No | ❗ |
| `UseIfInsteadOfWhen` | No | — |
| `UseIsNullOrEmpty` | Yes | ❗ |
| `UseLet` | No | ❗ |
| `UseOrEmpty` | No | ❗ |
| `UseRequire` | Yes | ❗ |
| `UseRequireNotNull` | Yes | ❗ |
| `UseSumOfInsteadOfFlatMapSize` | No | ❗ |
| `UtilityClassWithPublicConstructor` | Yes | — |
| `VarCouldBeVal` | Yes | ❗ |

> ⚠️ **Type Resolution requirement**: 116/226 rules (~51%) need Kotlin compiler type resolution.
> Pure Rust implementation may need alternative approaches or FFI bindings for these.

---

## Summary

| Layer | Total | ✅ Done | 🟡 Needs Tuning | ❌ Missing
|---|---|---|---|---|
| ktlint standard (JVM) | 105 | 61 | 3 | 41 |
| ktlint-rs-only | 18 | 18 | 0 | 0 |
| detekt comments | 9 | 0 | 0 | 9 |
| detekt complexity | 15 | 0 | 0 | 15 |
| detekt coroutines | 8 | 0 | 0 | 8 |
| detekt empty-blocks | 14 | 0 | 0 | 14 |
| detekt exceptions | 17 | 0 | 0 | 17 |
| detekt libraries | 3 | 0 | 0 | 3 |
| detekt naming | 21 | 0 | 0 | 21 |
| detekt performance | 10 | 0 | 0 | 10 |
| detekt potential-bugs | 37 | 0 | 0 | 37 |
| detekt ruleauthors | 2 | 0 | 0 | 2 |
| detekt style | 81 | ~5 | 0 | ~76 |
| **Total** | **340** | **~84** | **3** | **~253** |

> Note: detekt `formatting` and `ktlint` rule sets excluded (ktlint wrappers).
> ~8 detekt rules overlap with ktlint-rs (MaxLineLength, NoWildcardImports, etc.).
