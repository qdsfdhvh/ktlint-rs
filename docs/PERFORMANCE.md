# Performance

**Apple M2 · release build · rayon parallel**
Benchmarked 2026-07-12 with `scripts/bench.sh --release`.

> Optional detekt comparison: `brew install detekt` then `scripts/bench.sh --release`.
> Raw data in `bench_results.tsv`.

---

### compose-samples (380 files · 47K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 5K | 34 | 93% | 539ms | 86K/s | — |
| ktlint (JVM) | 13 | 10 | 2% | 12.15s | 4K/s | 23× slower |
| detekt | — | — | — | — | — | *(not installed)* |

### androidx (1,271 files · 267K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 49K | 53 | 100% | 820ms | 325K/s | — |
| ktlint (JVM) | 34K | 45 | 83% | 9.09s | 29K/s | 11× slower |
| detekt | — | — | — | — | — | *(not installed)* |

### nowinandroid (350 files · 31K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 4K | 40 | 89% | 201ms | 154K/s | — |
| ktlint (JVM) | 1K | 21 | 59% | 3.65s | 8K/s | 18× slower |
| detekt | — | — | — | — | — | *(not installed)* |

### okhttp (569 files · 131K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 26K | 45 | 92% | 528ms | 248K/s | — |
| ktlint (JVM) | 18 | 14 | 1% | 6.11s | 21K/s | 12× slower |
| detekt | — | — | — | — | — | *(not installed)* |

### ktor (2,478 files · 274K lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 48K | 47 | 93% | 2.31s | 119K/s | — |
| ktlint (JVM) | 355 | 27 | 3% | 10.34s | 26K/s | 4× slower |
| detekt | — | — | — | — | — | *(not installed)* |

### demo-gradle (8 files · 162 lines)

| Tool | Violations | Rules | Files w/ issues | Time | Throughput | vs JVM |
|---|---|---|---|---|---|---|
| **ktlint-rs** | 81 | 17 | 75% | 9ms | 18K/s | — |
| ktlint (JVM) | 167 | 18 | 75% | 2.11s | 77/s | 235× slower |
| detekt | — | — | — | — | — | *(not installed)* |

---

## Summary

| Metric | ktlint-rs | ktlint (JVM) |
|---|---|---|
| Total violations | 133K | 35K |
| Unique rules triggered | 74 | 54 |
| Total time | 4.41s | 43.45s |
| Aggregate throughput | 170K/s | 17K/s |
