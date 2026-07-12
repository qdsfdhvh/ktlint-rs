---
name: ktlint-rs
description: Use `ktlint` (the Rust rewrite) to lint and format Kotlin code — 17-25x faster than JVM ktlint. Covers spacing, structure, imports, naming, wrapping, and KDoc rules with auto-fix, .editorconfig support, and 4 reporters (plain, JSON, SARIF, summary). Drop-in compatible CLI.
---

# ktlint-rs

`ktlint` is a fast Kotlin linter and formatter written in Rust — drop-in
compatible with Pinterest's JVM-based [ktlint](https://github.com/pinterest/ktlint).
It uses tree-sitter to parse Kotlin source into a CST (preserving all whitespace
and comments), then checks 100+ rules across spacing, structure, imports,
naming, wrapping, and KDoc categories.  Auto-fix handles spacing violations;
parallel processing via rayon delivers 17-25x speedups over the JVM version.

Build from source before first use:

```bash
cd ktlint-rs && cargo build --release
```

The binary lives at `target/release/ktlint`.

## When to use ktlint-rs

```
Working on Kotlin code linting/formatting?
├─ No → not relevant
└─ Yes:
   ├─ Quick style check on a file/directory → ktlint <path>
   ├─ Pre-commit / CI gate → ktlint --reporter json <path> (exit code 1 = violations)
   ├─ Auto-fix spacing / formatting issues → ktlint --format <path>
   ├─ Machine-readable output for tooling → ktlint --reporter json|sarif <path>
   ├─ Quick summary of violations by rule → ktlint --reporter plain-summary <path>
   ├─ Disable specific rules for a file → @Suppress("ktlint:standard:<rule-id>")
   ├─ JVM ktlint parity check → ktlint --compat <path>
   ├─ Custom code style (android_studio / intellij_idea) → --code-style android_studio
   └─ Benchmark / compare vs JVM ktlint → time ktlint <path> && time java -jar ktlint <path>
```

## How it saves tokens vs manual review

| Naive approach | Better with ktlint-rs |
|---|---|
| Read `File.kt` and scan for style issues by eye | `ktlint File.kt` reports exact line:col violations |
| Grep for `^\s+$` to find trailing whitespace | `ktlint File.kt` catches trailing spaces + 99 other patterns |
| Manual formatting with search-replace | `ktlint --format File.kt` auto-fixes spacing in one pass |
| Open each file to check imports are sorted | `ktlint --reporter json src/` returns structured violations per rule |
| Read hundreds of lines to verify formatting | `ktlint src/ --limit 20` caps noise and returns only the first N violations |

**Output is agent-friendly:**
- Plain reporter (default): `file:line:col (rule-id) message` — greppable, structured.
- JSON reporter: `[{file, line, col, rule, message, auto_fixable}]` — parseable for tooling.
- SARIF reporter: standard format for CI integration.
- `--relative` prints paths relative to working directory.

## Commands

### Lint a file or directory

```bash
# Single file
ktlint path/to/File.kt

# Entire source tree (parallel via rayon)
ktlint src/

# Multiple paths
ktlint src/main/ src/test/
```

### Auto-fix

```bash
# Format all auto-fixable violations in-place
ktlint --format src/

# Format a single file
ktlint --format File.kt
```

Auto-fix handles: spacing around `{` `}` `=` `:` `,` `(` `)` operators, comment
spacing (`//`), blank lines before `}`, `} else` / `} catch` merging, trailing
spaces, and consecutive blank lines. Not all rules are auto-fixable — check
`auto_fixable` in JSON output.

### Reporters

```bash
# Plain text (default) — human-readable
ktlint src/

# JSON — parseable, includes auto_fixable field
ktlint --reporter json src/

# SARIF — CI integration
ktlint --reporter sarif src/

# Summary only — rule + count without file details
ktlint --reporter plain-summary src/
```

### Reporter output to file

```bash
ktlint --reporter json --reporter-output lint-results.json src/
```

### Limit output

```bash
# Show only first 20 violations
ktlint --limit 20 src/
```

### JVM ktlint compatibility mode

```bash
# Disable ktlint-rs-only rules for parity comparison
ktlint --compat src/

# Or via environment variable
KTLINT_COMPAT=1 ktlint src/
```

### Code style presets

```bash
# ktlint_official (default) — all rules enabled
ktlint src/

# android_studio — disables final-newline, wildcard-imports, import-ordering, trailing-comma, unused-imports
ktlint --code-style android_studio src/

# intellij_idea — disables wildcard-imports, import-ordering, trailing-comma
ktlint --code-style intellij_idea src/
```

### Relative paths

```bash
ktlint --relative src/
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
ktlint --editorconfig /path/to/custom/.editorconfig src/
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
| Spacing | 25 | curly, operator, comma, paren, colon, dot, keyword, annotation, modifier-order, double-colon |
| Structure | 30 | indent, trailing-space, blank-lines, max-line-length, trailing-comma, final-newline, enum-entry, no-empty-class-body |
| Imports | 5 | wildcard, ordering, unused |
| Naming | 8 | class, function, property, filename, package |
| Wrapping | 18 | chain, argument-list, multiline-if-else, try-catch, when-expression |
| KDoc | 6 | empty, first-line, trailing-space, no-blank |
| Comment | 5 | spacing, wrapping, block-comment, consecutive |
| Annotation | 3 | spacing, usage, @Suppress |

## Performance

**Benchmarks** (release build, Apple M2, rayon parallel):

| Project | Files | Lines | Time (ktlint-rs / JVM) |
|---|---|---|---|
| nowinandroid | 350 | 31,021 | **0.58s** / 10.1s (17x) |
| compose-samples | 380 | 46,586 | **0.61s** / 11.3s (18x) |
| okhttp | 569 | 131,098 | **0.87s** / 19.6s (22x) |
| androidx (26 modules) | 1,271 | 266,549 | **0.86s** / 21.9s (25x) |

## Test fixtures

Large Kotlin projects available for benchmarking and regression testing:

| Fixture | Files | Lines |
|---|---|---|
| `tests/fixtures/nowinandroid/` | 350 | 31K |
| `tests/fixtures/compose-samples/` | 380 | 47K |
| `tests/fixtures/okhttp/` | 569 | 131K |
| `tests/fixtures/androidx/` | 1,271 | 267K |
| `tests/fixtures/demo-gradle/` | 8 | ~200 |

Clone with submodules:

```bash
git clone --recurse-submodules https://github.com/qdsfdhvh/ktlint-rs.git
```

## Development workflow

```bash
# Run all 167+ tests
cargo test

# Run specific test
cargo test curly_spacing

# Run tests with output
cargo test -- --nocapture

# Build release
cargo build --release

# Lint the Rust source
cargo clippy

# Format Rust code
cargo fmt

# Self-test: lint the test fixtures
target/release/ktlint tests/fixtures/demo-gradle/
```

## Anti-patterns

- **Don't** use a JVM-based ktlint for speed-critical linting — ktlint-rs is 17-25x faster.
- **Don't** read files and manually review style when `ktlint <path>` gives exact line:col violations.
- **Don't** manually fix spacing issues one by one — `ktlint --format` handles 10 categories in one pass.
- **Don't** omit `--limit` on large projects — thousands of violations can flood output.
- **Don't** confuse `--compat` mode with full JVM parity — it only disables ktlint-rs–specific rules; per-rule counts still differ because JVM's `android_studio` preset is stricter.
- **Don't** forget to build after pulling — run `cargo build --release` or CI will use the stale binary.

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

Run `ktlint --help` for the full argument list. The README at
<https://github.com/qdsfdhvh/ktlint-rs> has current performance benchmarks
and the complete rule table.
