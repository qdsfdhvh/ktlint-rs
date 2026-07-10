# ktlint-rs

Use ktlint-rs to lint and format Kotlin code.

## Quick Usage

```bash
cd ktlint-rs && cargo build --release

# Lint all Kotlin files
target/release/ktlint src/

# Auto-fix spacing violations
target/release/ktlint --format src/

# JSON output
target/release/ktlint --reporter json path/to/File.kt

# Check specific directory
target/release/ktlint tests/fixtures/nowinandroid
```

## When linting/formatting Kotlin

- 每次修改 Kotlin 代码后运行: `ktlint --format`
- 提交前运行: `ktlint src/` 确保无违规
- CI 里用: `ktlint --reporter json src/`

## Speed

- 0.14s / 350 files (nowinandroid)
- 0.20s / 380 files (compose-samples)
- 15x faster than JVM ktlint
