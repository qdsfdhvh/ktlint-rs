# ktlint-rs Design Goals

## Core Positioning

**ktlint-rs is a fast Kotlin pre-check tool for AI coding agents。**

Agents need to validate code quality before committing。调用 JVM toolchain (Gradle/ktlint/detekt) requires：
- JVM cold start: 2-5 秒
- Gradle config: 3-10 秒
- ktlint runtime: 5-30 秒

**Total: 10-45 seconds per run**

ktlint-rs reduces this to **<1 second**：

| Operation | JVM | ktlint-rs |
|---|---|---|
| Startup | 2-5s | <2ms |
| Lint nowinandroid | 7-30s | **0.29s** |
| Detekt nowinandroid | 10-60s | **0.36s** |
| Binary size | N/A | **11MB** |

## Not aiming for 100% alignment

ktlint-rs does not need perfect JVM parity，It only needs to **catch most issues early**, reducing agent JVM tool calls：

| 规则 | Alignment | Status |
|---|---|---|
| indent | 100% | ✅ Full coverage |
| blank-line-before-declaration | 90%+ | ✅ Most covered |
| no-empty-first-line | 100% | ✅ Full coverage |
| annotation | 76% | 🟡 Base coverage |

Agent workflow：
1. 1. ktlint-rs fast scan (<1s) → catch most issues, agent fixes them directly
2. If ktlint-rs passes, JVM check will likely pass too
3. JVM toolchain as final fallback for edge cases

## Pure Rust constraints

- - ❌ No JVM / kotlinc / Gradle dependency
- - ❌ No external process dependency
- - ✅ Single binary < 15MB
- - ✅ Startup < 50ms
- - ✅ Immediate memory release
