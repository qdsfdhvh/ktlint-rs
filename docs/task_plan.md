# Task Plan — Issue #71: Production-readiness blockers

> Goal: ktlint-rs v0.1.5 passes JVM ktlint 1.8.0 parity on Repro.kt + large KMP corpus

## Blocker 1: JVM-clean Kotlin produces false positives ✅→🔧

| # | Rule | Repro.kt line | Fix | Status |
|---|------|--------------|-----|--------|
| 1 | `spacing-around-angle-brackets` | L6 (`count > 0`) | Only flag `<`/`>` inside `type_arguments`/`type_parameters` CST nodes | ❌ |
| 2 | `spacing-around-angle-brackets` | L38 (`0 < 1` comment) | Skip comments (CST context) | ❌ |
| 3 | `colon-spacing` | L16 (`catch (e: Type)`) | Skip `:` in catch parameter (type annotation, no spacing violation) | ❌ |
| 4 | `spacing-around-double-colon` | L28 (`::println`) | Skip callable references `::` (different from double-colon spacing) | ❌ |
| 5 | `spacing-around-keyword` | L31 (`DebugMenuEntry` contains `try`) | Word-boundary check before keyword match | ✅ fixed |
| 6 | `no-multi-spaces` | L33 (`"   "` in string) | Mask string literal byte spans | ✅ written, needs verify |
| 7 | `class-naming` | L1 (`enum class AuthAction`) | Skip `enum` prefix before `class` | ❌ |
| 8 | `multiline-if-else` | L10 (`else ->`) | Skip `else ->` in when-body | ❌ |
| 9 | `kdoc` | L35 (KDoc on private) | Skip KDoc on private/private-like declarations | ❌ |

## Blocker 2: per-file EditorConfig discards CLI configuration

- `KtlintConfig::load_for_file()` creates default config, losing CLI overrides
- Fix: `load_for_file` must accept a base config and overlay per-file EditorConfig
- Affected: `--ruleset`, `--compat`, `--code-style`, `--editorconfig`, `--config`
- Need matrix integration test

## Blocker 3: cache staleness

- EditorConfig changes don't invalidate cache (fingerprint missing effective rules)
- Same-size same-mtime edits collide (need content hash)
- Fix: add content hash + full config fingerprint

## Blocker 4: formatter doesn't converge

- `--format` exits 1 based on pre-format state
- Need to re-lint post-format and report post-format state
- Second pass must be idempotent

## Blocker 5: unmatched input is silent success

- `ktlint-rs does-not-exist` → exit 0, no warning
- Fix: print warning, add `--strict` option for CI
