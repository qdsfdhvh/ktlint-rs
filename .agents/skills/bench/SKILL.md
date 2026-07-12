# bench — Performance benchmark & parity report

Runs ktlint-rs vs ktlint JVM on real-world Kotlin projects. Generates
multi-dimensional comparison across violations, timing, rules, and files.

## Usage

```bash
# Setup fixtures + ktlint jar:
./scripts/setup-fixtures.sh
./scripts/get-ktlint.sh

# Release mode (for final numbers):
./scripts/bench.sh --release

# Debug mode (faster build, verify):
./scripts/bench.sh
```

## Output: 4-Part Report

### 1. Summary Table

```
Project                 Files   Lines     Violations(rs/JVM)  Time(rs/JVM)    Speedup
----------------------------------------------------------------------------------------
nowinandroid             350    31,021    5,062 / 1,038       0.23s / 6.94s    30x
compose-samples (6)      380    46,586    5,258 / 13          0.31s / 6.93s    22x
okhttp                   569   131,098    33,001 / 18          1.25s / 8.16s     7x
androidx (26 mods)      1271   266,549    TBD / 33,731        TBD / 10.6s       TBD
```

### 2. Per-Rule Breakdown

```
Rule                                           ktlint-rs  ktlint JVM  Status
-------------------------------------------------------------------------------
standard:indent                                  3000          15      [~]
standard:colon-spacing                            196           0      [rs]
standard:no-empty-first-line-in-class-body        670         107      [~]
standard:filename                                   0           1      [jvm]
...
-------------------------------------------------------------------------------
Rules total: 35   ✓ matched: 14   ~ diff: 6   rs-only: 10   jvm-only: 5
```

Status: ✓ exact match | ~ both but different | rs only ktlint-rs | jvm only JVM

### 3. Parity Scorecard

```
Metric                      ktlint-rs      ktlint JVM     Match
------------------------------------------------------------------
Exit code                   1              1              ✓
Files scanned               310            310            ✓
Rules triggered             23             21             ~
Rules perfect match         14             -              ~
Rules rs-only               10             -              ✗
Rules jvm-only              -              5              ✗
Total violations            5,062          1,038          ✗
Time                        0.23s          6.94s          ✓ (faster)
```

### 4. Per-Project Detail

```
nowinandroid:
  Files: 350 (.kt + .kts)
  Lines: 31,021
  Violations: rs=5,062  jvm=1,038
  Rules: rs=23  jvm=21  matched=14  rs-only=10  jvm-only=5
  Top rs-only rules: colon-spacing(196), op-spacing(41), ...
  Top jvm-only rules: filename(1), kdoc(5), wrapping(5), ...
```

## How to read the scorecard

| Metric | Meaning | Target |
|---|---|---|
| Exit code | Both tools agree on success/failure | ✓ match |
| Files scanned | Same .kt/.kts files discovered | ✓ match |
| Rules triggered | How many unique rules fire | Should converge |
| Rules perfect match | Rules with identical violation counts | Should increase |
| Time | Execution speed | ktlint-rs should be faster |

## Adding new comparison dimensions

To add a dimension, edit `scripts/bench.sh`:
1. Add a new section after the existing ones (e.g., `## Formatter Diff`)
2. Use `run_tool rs` / `run_tool jvm` helpers
3. Write results to stdout

## Prerequisites

- `java` (JRE) for ktlint JVM
- `./scripts/get-ktlint.sh` (downloads ktlint fat jar)
- `./scripts/setup-fixtures.sh` (shallow-clones test repos)
