# ktlint-rs CLI Commands

## Lint

```bash
# Lint all Kotlin files in current directory
ktlint-rs **/*.kt

# Lint with ktlint rules only
ktlint-rs --ruleset ktlint

# Lint with detekt rules only  
ktlint-rs --ruleset detekt

# Lint both (default)
ktlint-rs --ruleset ktlint,detekt
```

## Format (auto-fix)

```bash
# Auto-fix spacing, wrapping, indentation violations
ktlint-rs --format **/*.kt
```

## Output

```bash
# Default (plain text)
ktlint-rs **/*.kt

# JSON output
ktlint-rs --reporter json **/*.kt

# SARIF (CI integration)
ktlint-rs --reporter sarif **/*.kt

# Checkstyle
ktlint-rs --reporter checkstyle **/*.kt

# Markdown
ktlint-rs --reporter markdown **/*.kt

# Write to file
ktlint-rs --reporter json --reporter-output report.json **/*.kt
```

## Configuration

```bash
# Specify .editorconfig path
ktlint-rs --editorconfig .editorconfig

# YAML config (detekt-style)
ktlint-rs --config ktlint-rs.yml

# Code style preset
ktlint-rs --code-style android_studio
ktlint-rs --code-style intellij_idea
ktlint-rs --code-style ktlint_official
```

## Baseline

```bash
# Generate baseline from current violations
ktlint-rs --create-baseline

# Filter violations against baseline
ktlint-rs --baseline baseline.xml
```

## Other

```bash
# Print version
ktlint-rs --version

# Limit output
ktlint-rs --limit 10

# Colorize output
ktlint-rs --color

# Show relative paths
ktlint-rs --relative
```
