---
name: ktlint-rs
description: Use `ktlint-rs` to lint and format Kotlin code — 10-27x faster than JVM ktlint. 264 rules, auto-fix, .editorconfig support, Spotless 8.8.0 parity. Drop-in compatible CLI.
---

# ktlint-rs

`ktlint-rs` is a fast Kotlin linter and formatter written in Rust — drop-in
compatible with Pinterest's JVM-based [ktlint](https://github.com/pinterest/ktlint),
including Spotless 8.8.0 `spotlessApply` parity. It uses tree-sitter to parse
Kotlin source into a CST (preserving all whitespace and comments), then checks
**264 registry rules** (116 standard/ktlint-oriented + 148 detekt) across spacing,
structure, imports, naming, wrapping, and KDoc. Auto-fix handles spacing and
indentation; parallel processing via rayon delivers 10-27x speedups over the JVM
version.

```bash
cd ktlint-rs && cargo build --release    # binary at target/release/ktlint-rs
```

## When to use ktlint-rs

```
Working on Kotlin code linting/formatting?
├─ No → not relevant
└─ Yes:
   ├─ Quick style check → ktlint-rs <path>
   ├─ Pre-commit / CI gate → ktlint-rs --reporter json <path>
   ├─ Auto-fix → ktlint-rs --format <path>
   ├─ Spotless replacement → ktlint-rs --format + check (matches spotlessApply byte-for-byte)
   ├─ Structured output → ktlint-rs --reporter json|sarif <path>
   ├─ Summary by rule → ktlint-rs --reporter plain-summary <path>
   ├─ Disable rules → @Suppress("ktlint:standard:<rule-id>") or .editorconfig
   ├─ Code style → ktlint-rs --code-style android_studio <path>
   └─ Benchmark → time ktlint-rs <path> && time ktlint <path>
```

## Commands

### Lint

```bash
ktlint-rs path/to/File.kt             # single file
ktlint-rs src/                        # directory (parallel via rayon)
ktlint-rs --ruleset ktlint src/       # ktlint rules only (excludes detekt)
ktlint-rs --include '**/src/**/*.kt' --exclude '**/generated/**' .  # glob scope
```

### Auto-fix

```bash
ktlint-rs --format src/               # format in-place
ktlint-rs --format File.kt            # single file
```

Handles: `{ } = : , ( )` spacing, comment spacing, blank lines before `}`,
`} else` / `} catch` merging, trailing spaces, consecutive blank lines, **indent
fix (zero-indent lines inside blocks)**, **when-branch blank lines (block bodies)**,
**trailing-lambda parens (`forEach()` → `forEach`)**.

### Reporters

```bash
ktlint-rs src/                        # plain text (default)
ktlint-rs --reporter json src/        # JSON, includes auto_fixable
ktlint-rs --reporter sarif src/       # CI integration
ktlint-rs --reporter plain-summary src/  # rule counts only
ktlint-rs --reporter json --reporter-output lint.json src/
```

## Configuration (.editorconfig)

```ini
[*.{kt,kts}]
ktlint_code_style = ktlint_official
indent_size = 4
indent_style = space
max_line_length = 120
insert_final_newline = true
trim_trailing_whitespace = true
ktlint_standard_no_wildcard_imports = disabled
ktlint_standard_trailing_comma = enabled
```

```bash
ktlint-rs --editorconfig /path/to/custom/.editorconfig src/
```

## @Suppress support

```kotlin
@file:Suppress("ktlint:standard:final-newline")
@Suppress("ktlint:standard:curly-spacing", "ktlint:standard:no-wildcard-imports")
class Foo { }

@Suppress("ktlint:standard:max-line-length")
val x = "a very long string..."
```

## Rules (264 total)

| Category | Count | Notes |
|---|---|---|
| ktlint standard | 116 | parity with ktlint 1.8 (101 oracle rules matched, 0 missing) |
| detekt | 148 | registry compatibility |

Verified against live projects (ktor 2311 files, nowinandroid 310, okhttp 525,
compose-samples 355): **zero false positives** on a 40-file differential sample
against the real ktlint 1.8.0 CLI, and `--format` changes **0 files** on
already-formatted code.

## Known behavior notes

- Rules that ktlint 1.8 disables by default (e.g. `expression-operand-wrapping`)
  stay off unless the .editorconfig explicitly enables them.
- Some rules are fail-closed (no reporting) until a CST implementation lands;
  they are safe but conservative.
- The `blank-line-between-when-conditions` rule separates block-body branches
  from previous branches (simple-expression branches stay adjacent).

## Anti-patterns
- **Don't** use JVM ktlint for speed-critical linting — ktlint-rs is 10-27x faster.
- **Don't** manually scan files for style issues — `ktlint-rs <path>` prints each violation as `path:line:col (rule) message` (1-based line and column).
- **Don't** fix spacing one by one — `ktlint-rs --format` handles it in one pass.
- **Don't** omit `--limit` on large projects — thousands of violations can flood output.
- **Don't** forget to build after pulling — `cargo build --release`.

Run `ktlint-rs --help` for full argument list. Source at [qdsfdhvh/ktlint-rs](https://github.com/qdsfdhvh/ktlint-rs).
