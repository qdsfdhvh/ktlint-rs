# ktlint-rs Design Goals

## Core Positioning

**ktlint-rs is a fast Kotlin pre-check tool for AI coding agents.**

Agents need to validate code quality before committing. Calling JVM toolchain (Gradle/ktlint/detekt) requires:
- JVM cold start: 2-5s
- Gradle config: 3-10s
- ktlint runtime: 5-30s

**Total: 10-45 seconds per run**

ktlint-rs reduces this to **<1 second**:

| Operation | JVM | ktlint-rs |
|---|---|---|
| Startup | 2-5s | <2ms |
| Lint nowinandroid | 7-30s | **0.29s** |
| Detekt nowinandroid | 10-60s | **0.36s** |
| Binary size | N/A | **11MB** |

## Not aiming for 100% alignment

ktlint-rs does not need perfect JVM parity. It only needs to **catch most issues early**, reducing agent JVM tool calls:

| Rule | Alignment | Status |
|---|---|---|
| indent | 100% | ✅ Full coverage |
| blank-line-before-declaration | 90%+ | ✅ Most covered |
| no-empty-first-line | 100% | ✅ Full coverage |
| annotation | 76% | 🟡 Base coverage |

Agent workflow:
1. ktlint-rs fast scan (<1s) → catch most issues, agent fixes them directly
2. If ktlint-rs passes, JVM check will likely pass too
3. JVM toolchain as final fallback for edge cases

## Spotless 8.8.0 / ktlint 1.8.0 parity

Since the differential harness landed, parity is verified continuously rather
than approximated:

- A pinned Spotless 8.8.0 + ktlint 1.8.0 oracle (101 standard rules) is diffed
  byte-for-byte on every PR: discovery, effective config, diagnostics (incl.
  autocorrectability), exit codes, formatted bytes, and idempotence all match.
- Live projects are checked against the real ktlint CLI with each project's
  `.editorconfig`: a 40-file ktor sample shows **zero false positives**
  (ktlint-rs is conservative — it reports 4 real violations vs ktlint's 19).
- `--format` leaves already-formatted code untouched (0 files changed on
  ktor/nowinandroid/kataris-app) and matches `spotlessApply` byte-for-byte on
  unformatted new code (indent, when-branch blank lines, trailing-lambda parens).

## Pure Rust constraints

- ❌ No JVM / kotlinc / Gradle dependency
- ❌ No external process dependency
- ✅ Single binary < 30MB
- ✅ Startup < 50ms
- ✅ Immediate memory release
