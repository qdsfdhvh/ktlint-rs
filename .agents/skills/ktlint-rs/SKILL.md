---
name: ktlint-rs
description: Use `ktlint-rs` — a pure-Rust Kotlin linter & formatter (drop-in compatible with JVM ktlint). 65 rules, auto-fix, .editorconfig, 4 reporters. 10-27x faster than JVM.
---

# ktlint-rs

`ktlint-rs` is a fast Kotlin linter and formatter written in Rust — drop-in
compatible with Pinterest's JVM-based [ktlint](https://github.com/pinterest/ktlint).
It uses tree-sitter to parse Kotlin source into a CST (preserving all whitespace
and comments), then checks 65 rules across spacing, structure, imports, naming,
wrapping, and KDoc. Parallel processing via rayon delivers 10-27x speedups.

## Installation

```bash
cargo install ktlint-rs
```

## Commands

```bash
# Lint
ktlint-rs src/                        # parallel via rayon
ktlint-rs path/to/File.kt             # single file

# Auto-fix
ktlint-rs --format src/

# Reporters
ktlint-rs --reporter json src/        # structured output
ktlint-rs --reporter sarif src/       # CI integration
ktlint-rs --reporter plain-summary src/  # rule counts only

# Code style
ktlint-rs --code-style android_studio src/
ktlint-rs --code-style intellij_idea src/

# Limits & paths
ktlint-rs --limit 20 src/
ktlint-rs --relative src/
```

Auto-fix handles: spacing around `{ } = : , ( )` operators, comment spacing,
blank-line fixes, and trailing spaces.

## Configuration (.editorconfig)

ktlint-rs reads `.editorconfig` from the project directory:

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

## Performance

**Benchmarks** (release build, Apple M2, rayon parallel):

| Project | Files | Lines | Violations (rs / JVM) | Time (rs / JVM) |
|---|---|---|---:|---:|
| nowinandroid | 350 | 31,021 | 9,901 / 1,038 | **0.26s** / 6.71s |
| compose-samples (6 apps) | 380 | 46,586 | 10,752 / 13 | **0.30s** / 7.96s |
| okhttp | 569 | 131,098 | 40,632 / 18 | **1.19s** / 11.5s |
| androidx (26 modules) | 1,271 | 266,549 | 86,591 / 33,731 | **1.07s** / 10.6s |

## Anti-patterns

- **Don't** use JVM ktlint for speed-critical linting — ktlint-rs is 10-27x faster.
- **Don't** manually scan files for style issues — `ktlint-rs <path>` gives exact line:col violations.

Run `ktlint-rs --help` for full argument list. Source at [qdsfdhvh/ktlint-rs](https://github.com/qdsfdhvh/ktlint-rs).
