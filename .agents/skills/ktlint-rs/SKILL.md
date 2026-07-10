# ktlint-rs — Kotlin Linting Skill

Use ktlint-rs to lint and format Kotlin code. Drop-in replacement for ktlint.

## When to use

- 检查 Kotlin 代码风格时: `ktlint src/`
- 自动修复格式问题: `ktlint --format src/`
- 提交前检查: `ktlint --reporter json src/`
- 对比 JVM ktlint 准确度时: run both and diff

## Commands

```bash
# Build (first time only)
cd ktlint-rs && cargo build --release

# Lint a project directory
target/release/ktlint tests/fixtures/nowinandroid

# Lint a single file
target/release/ktlint path/to/File.kt

# Auto-fix spacing violations
target/release/ktlint --format path/to/File.kt

# JSON reporter (for CI/tooling)
target/release/ktlint --reporter json src/

# Lint multiple directories
target/release/ktlint tests/fixtures/nowinandroid tests/fixtures/compose-samples
```

## EditorConfig support

ktlint-rs reads `.editorconfig` from the project directory. Supports:
- `indent_size`, `indent_style`, `max_line_length`
- `ktlint_code_style` (ktlint_official / android_studio / intellij_idea)
- `ktlint_standard_<rule-id>` = enabled / disabled
- `insert_final_newline`, `trim_trailing_whitespace`

## @Suppress support

```kotlin
@Suppress("ktlint:standard:curly-spacing")
class Foo{ } // curly-spacing violations suppressed here
```

## Test fixtures

Large Kotlin projects available for benchmarking:
- `tests/fixtures/nowinandroid/` — 350 files, 29K lines
- `tests/fixtures/compose-samples/` — 380 files, 45K lines
- `tests/fixtures/androidx/` — AndroidX source
- `tests/fixtures/demo-gradle/` — 8-file demo project

Clone with: `git clone --recurse-submodules https://github.com/qdsfdhvh/ktlint-rs.git`

## Speed

0.14s on nowinandroid (350 files) — 15x faster than JVM ktlint (2.17s)
