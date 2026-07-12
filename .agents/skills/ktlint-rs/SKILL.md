---
name: ktlint-rs
description: Use `ktlint-rs` — a pure-Rust Kotlin linter & formatter (drop-in compatible with JVM ktlint). 78 rules across spacing, structure, imports, naming, wrapping, and KDoc. Auto-fix, .editorconfig, 4 reporters. 17-25x faster than JVM.
---

# ktlint-rs

`ktlint-rs` is a fast Kotlin linter and formatter written in Rust — drop-in
compatible with Pinterest's JVM-based [ktlint](https://github.com/pinterest/ktlint).
It uses tree-sitter to parse Kotlin source into a CST (preserving all whitespace
and comments), then checks 78 rules across spacing, structure, imports,
naming, wrapping, and KDoc categories.  Auto-fix handles spacing violations;
parallel processing via rayon delivers 17-25x speedups over the JVM version.

## Installation

```bash
cargo install ktlint-rs
```

The binary will be available as `ktlint-rs`.

## When to use ktlint-rs

```
Working on Kotlin code linting/formatting?
├─ No → not relevant
└─ Yes:
   ├─ Quick style check on a file/directory → ktlint-rs <path>
   ├─ Pre-commit / CI gate → ktlint-rs --reporter json <path> (exit code 1 = violations)
   ├─ Auto-fix spacing / formatting issues → ktlint-rs --format <path>
   ├─ Machine-readable output for tooling → ktlint-rs --reporter json|sarif <path>
   ├─ Quick summary of violations by rule → ktlint-rs --reporter plain-summary <path>
   ├─ Custom code style (android_studio / intellij_idea) → --code-style android_studio
```

## How it saves tokens vs manual review

| Naive approach | Better with ktlint-rs |
|---|---|
| Read `File.kt` and scan for style issues by eye | `ktlint-rs File.kt` reports exact line:col violations |
| Grep for `^\s+$` to find trailing whitespace | `ktlint-rs File.kt` catches trailing spaces + 77 other patterns |
| Manual formatting with search-replace | `ktlint-rs --format File.kt` auto-fixes spacing in one pass |
| Open each file to check imports are sorted | `ktlint-rs --reporter json src/` returns structured violations per rule |

**Output is agent-friendly:**
- Plain reporter (default): `file:line:col (rule-id) message` — greppable, structured.
- JSON reporter: `[{file, line, col, rule, message, auto_fixable}]` — parseable for tooling.
- SARIF reporter: standard format for CI integration.
- `--relative` prints paths relative to working directory.

## Commands

### Lint a file or directory

```bash
# Single file
ktlint-rs path/to/File.kt

# Entire source tree (parallel via rayon)
ktlint-rs src/

# Multiple paths
ktlint-rs src/main/ src/test/
```

### Auto-fix

```bash
# Format all auto-fixable violations in-place
ktlint-rs --format src/

# Format a single file
ktlint-rs --format File.kt
```

Auto-fix handles: spacing around `{` `}` `=` `:` `,` `(` `)` operators, comment
spacing (`//`), blank lines before `}`, `} else` / `} catch` merging, trailing
spaces, and consecutive blank lines. Not all rules are auto-fixable — check
`auto_fixable` in JSON output.

### Reporters

```bash
# Plain text (default) — human-readable
ktlint-rs src/

# JSON — parseable, includes auto_fixable field
ktlint-rs --reporter json src/

# SARIF — CI integration
ktlint-rs --reporter sarif src/

# Summary only — rule + count without file details
ktlint-rs --reporter plain-summary src/
```

### Reporter output to file

```bash
ktlint-rs --reporter json --reporter-output lint-results.json src/
```

### Limit output

```bash
# Show only first 20 violations
ktlint-rs --limit 20 src/
```


### Code style presets

```bash
# ktlint_official (default) — all rules enabled
ktlint-rs src/

# android_studio — disables final-newline, wildcard-imports, import-ordering, trailing-comma, unused-imports
ktlint-rs --code-style android_studio src/

# intellij_idea — disables wildcard-imports, import-ordering, trailing-comma
ktlint-rs --code-style intellij_idea src/
```

### Relative paths

```bash
ktlint-rs --relative src/
```

## Configuration (.editorconfig)

ktlint-rs reads `.editorconfig` from the project directory:

```ini
[*.{kt,kts}]
# Code style profile
ktlint_code_style = ktlint_official

# Indentation
indent_size = 4
indent_style = space

# Line rules
max_line_length = 120           # 0 = off
insert_final_newline = true
trim_trailing_whitespace = true

# Per-rule enable/disable
ktlint_standard_no_wildcard_imports = enabled
ktlint_standard_curly_spacing = enabled
ktlint_standard_trailing_comma = disabled
```

You can also specify a custom editorconfig path:

```bash
ktlint-rs --editorconfig /path/to/custom/.editorconfig src/
```

## @Suppress support

Suppress violations at file level, class level, or statement level:

```kotlin
// File-level suppression
@file:Suppress("ktlint:standard:final-newline")

// Class-level suppression
@Suppress("ktlint:standard:curly-spacing", "ktlint:standard:no-wildcard-imports")
class Foo { }

// Statement-level suppression
@Suppress("ktlint:standard:max-line-length")
val x = "a very long string that exceeds the configured max line length"
```

## Rules overview

| Category | Count | Examples |
|---|---|---|
| Spacing | 17 | curly, operator, comma, paren, colon, dot, keyword, annotation, modifier-order |
| Structure | 27 | indent, trailing-space, blank-lines, max-line-length, trailing-comma, enum-entry |
| Imports | 4 | wildcard, ordering, unused |
| Naming | 6 | class, function, property, filename, package |
| Wrapping | 7 | chain, multiline-if-else, try-catch, when-expression |
| KDoc | 4 | formatting, no-empty, no-trailing |
| Plus | 13 | Built-in + Phase/Final rules |
| **Total** | **78** | |

## Performance

**Benchmarks** (release build, Apple M2, rayon parallel):

| Project | Files | Lines | Time (ktlint-rs / JVM) |
|---|---|---|---|
| compose-samples | 380 | 46,586 | **0.30s** / 7.96s (26x) |
| nowinandroid | 350 | 31,021 | **0.26s** / 6.71s (25x) |
| okhttp | 569 | 131,098 | **1.19s** / 11.5s (10x) |
| androidx | 1,271 | 266,549 | **1.07s** / 10.6s (10x) |


## Anti-patterns

- **Don't** use a JVM-based ktlint for speed-critical linting — ktlint-rs is 17-25x faster.
- **Don't** read files and manually review style when `ktlint-rs <path>` gives exact line:col violations.
- **Don't** omit `--limit` on large projects — thousands of violations can flood output.

## Reporting pain points

When using ktlint-rs on a project task, keep track of concrete tool pain points
such as false-positive violations, missing rules, incorrect auto-fix output,
confusing messages, or performance regressions. If the user allows or requests
upstream feedback, file an issue against `qdsfdhvh/ktlint-rs` with a small
sanitized repro and the expected/actual behavior.

**Privacy rule**: never include the current project's repo name, file paths,
package names, class/function names, logs, business terminology, screenshots,
or source snippets in upstream issues unless the user explicitly provides
public repro material. Rewrite examples as generic placeholders like
`example-domain`, `FeatureViewModel`, and `path/to/File.kt`; prefer minimal
synthetic code that reproduces the tool behavior.

## Help

Run `ktlint-rs --help` for the full argument list. The README at
<https://github.com/qdsfdhvh/ktlint-rs> has current performance benchmarks
and the complete rule table.
