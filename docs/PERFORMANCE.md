# Performance

**Apple M2 · release build · rayon parallel**
Benchmarked 2026-07-15.

### Performance Improvements (Jul 2026)
- **O(1) engine init**: RuleEngine built once (was per-file)
- **Iterative CST walks**: no stack overflow on deep files
- **Incremental cache**: 2.3× speedup on repeated runs (`.cache/ktlint-rs/cache.json`)
- **Scoped rayon pool**: threads exit after lint, CPU → 0

> detekt comparison: `brew install detekt` (requires `--all-rules` + minimal config).
> Raw data in `bench_results.tsv`.

---

### compose-samples (380 files · 47K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 5K | 34 | 93% | 175ms | 266K/s | — |
| ktlint (JVM) | 13 | 10 | 2% | 3.34s | 14K/s | 19× slower |
| detekt | 3K | 28 | — | 5.68s | 8K/s | 32× slower |

### androidx (1,271 files · 267K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 49K | 53 | 100% | 1.05s | 253K/s | — |
| ktlint (JVM) | 34K | 45 | 83% | 10.14s | 26K/s | 10× slower |
| detekt | 130K | 73 | — | 423.31s | 630/s | 402× slower |

### nowinandroid (350 files · 31K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 4K | 40 | 89% | 142ms | 218K/s | — |
| ktlint (JVM) | 1K | 21 | 59% | 3.83s | 8K/s | 27× slower |
| detekt | 440 | 20 | — | 3.98s | 8K/s | 28× slower |

### okhttp (569 files · 131K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 26K | 45 | 92% | 535ms | 245K/s | — |
| ktlint (JVM) | 18 | 14 | 1% | 6.26s | 21K/s | 12× slower |
| detekt | 2K | 38 | — | 13.87s | 9K/s | 26× slower |

---

> ⚠️ **detekt on androidx**: 423s (7 min) with `--all-rules`. The 130K violations show detekt's broader scope (static analysis, not just formatting).
> ktor and demo-gradle omitted — bench timed out before completion.

---

### Post-parity benchmark (2026-08) — after differential fixes

Violation counts dropped dramatically once false positives were eliminated
(PRs #96-#98); timing is unchanged:

| Project | Files | ktlint-rs check | Violations now | Notes |
|---|---|---|---|---|
| ktor | 2311 | 4.3s | 490 | mostly real (ktor doesn't run ktlint in CI) |
| kataris-app | 1949 | 3.7s | 1 | **matches Spotless exactly** |
| nowinandroid | 310 | 1.2s | 1 (real) | ktlint 1.4 codebase |

`--format` on already-formatted code changes **0 files** (ktor/nowinandroid/kataris-app)
and matches `spotlessApply` byte-for-byte on unformatted new code.

> Earlier rows above reflect pre-fix behavior (mass false positives from
> placeholder line-scan rules); see git history #96-#98 for the fixes.
