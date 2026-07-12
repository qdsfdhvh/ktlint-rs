---
name: ktlint-rs
description: Use `ktlint-rs` to lint and format Kotlin code — 10-27x faster than JVM ktlint. 65 rules, auto-fix, .editorconfig support, 4 reporters. Drop-in compatible CLI.
---

# ktlint-rs

`ktlint-rs` is a fast Kotlin linter and formatter written in Rust — drop-in
compatible with Pinterest's JVM-based [ktlint](https://github.com/pinterest/ktlint).
It uses tree-sitter to parse Kotlin source into a CST (preserving all whitespace
and comments), then checks 65 rules across spacing, structure, imports, naming,
wrapping, and KDoc. Auto-fix handles spacing violations; parallel processing via
rayon delivers 10-27x speedups over the JVM version.

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
   ├─ Structured output → ktlint-rs --reporter json|sarif <path>
   ├─ Summary by rule → ktlint-rs --reporter plain-summary <path>
   ├─ Disable rules → @Suppress("ktlint:standard:<rule-id>")
   ├─ JVM parity check → ktlint-rs --compat <path>
   ├─ Code style → ktlint-rs --code-style android_studio <path>
   └─ Benchmark → time ktlint-rs <path> && time ktlint <path>
```

## Commands

### Lint

```bash
ktlint-rs path/to/File.kt             # single file
ktlint-rs src/                        # directory (parallel via rayon)
ktlint-rs src/main/ src/test/         # multiple paths
```

### Auto-fix

```bash
ktlint-rs --format src/               # format in-place
ktlint-rs --format File.kt            # single file
```

Handles: `{ } = : , ( )` spacing, comment spacing, blank lines before `}`, `} else` / `} catch` merging, trailing spaces, consecutive blank lines.

### Reporters

```bash
ktlint-rs src/                        # plain text (default)
ktlint-rs --reporter json src/        # JSON, includes auto_fixable
ktlint-rs --reporter sarif src/       # CI integration
ktlint-rs --reporter plain-summary src/  # rule counts only
ktlint-rs --reporter json --reporter-output lint.json src/
```

### Limits & options

```bash
ktlint-rs --limit 20 src/             # first 20 violations
ktlint-rs --relative src/             # relative paths
ktlint-rs --compat src/               # disable rs-only rules
ktlint-rs --code-style android_studio src/
ktlint-rs --code-style intellij_idea src/
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

## Rules (65 total)

| Category | Count | Examples |
|---|---|---|
| Spacing | 17 | curly, operator, comma, paren, colon, dot, keyword, annotation, modifier-order |
| Structure | 28 | indent, trailing-space, blank-lines, max-line-length, trailing-comma, kdoc |
| Imports | 4 | wildcard, ordering, unused |
| Naming | 6 | class, function, property, filename, package |
| Wrapping | 7 | chain, multiline-if-else, try-catch, when-expression |
| KDoc | 3 | formatting, no-empty, no-trailing |

## Anti-patterns
## Anti-patterns

- **Don't** use JVM ktlint for speed-critical linting — ktlint-rs is 10-27x faster.
- **Don't** manually scan files for style issues — `ktlint-rs <path>` gives exact line:col.
- **Don't** fix spacing one by one — `ktlint-rs --format` handles it in one pass.
- **Don't** omit `--limit` on large projects — thousands of violations can flood output.
- **Don't** forget to build after pulling — `cargo build --release`.

Run `ktlint-rs --help` for full argument list. Source at [qdsfdhvh/ktlint-rs](https://github.com/qdsfdhvh/ktlint-rs).
