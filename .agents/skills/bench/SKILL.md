# bench — Performance benchmark & table generator

Runs ktlint-rs vs ktlint JVM on real-world Kotlin projects and generates
per-rule breakdowns plus summary tables.

## Usage

```bash
# Setup fixtures first:
./scripts/setup-fixtures.sh

# Release mode (for final numbers):
./scripts/bench.sh --release

# Debug mode (faster build, for verifying):
./scripts/bench.sh
```

## Output

1. **Summary Table** — Files / Lines / Violations / Time (rs vs JVM):

```
Project                            Files  Lines    Violations (rs/JVM)  Time (rs/JVM)
------------------------------------------------------------------------------------------
nowinandroid                        350   31,021    9,901 / 1,038        0.26s / 6.71s
compose-samples (6 apps)            380   46,586   10,752 / 13           0.30s / 7.96s
okhttp                              569  131,098   40,632 / 18           1.19s / 11.5s
androidx (26 modules)              1271  266,549   86,591 / 33,731       1.07s / 10.6s
```

2. **Per-Rule Breakdown** — every rule with counts + match status:

```
Rule                                           ktlint-rs  ktlint JVM
------------------------------------------------------------------
standard:indent                                  6948         15    [~]
standard:blank-line-before-declaration           1240         25    [~]
standard:colon-spacing                            196          0    [rs]
standard:filename                                   0          1    [jvm]
...
Rules total: 35  ✓ matched: 14  ~ diff: 6  rs-only: 10  jvm-only: 5
```

3. **Exit Codes** — per-project comparison.

## Prerequisites

- `ktlint` JVM CLI installed (`brew install ktlint`)
- `./scripts/setup-fixtures.sh` (shallow clones into `tests/fixtures/`)
- For AndroidX: 26 selected first-level modules (see `ANDROIDX_DIRS` in script)

## Fixtures

| Fixture | Source | Files | Lines |
|---|---|---|---|
| nowinandroid | github.com/android/nowinandroid | 350 | 31K |
| compose-samples | github.com/android/compose-samples | 380 | 47K |
| okhttp | github.com/square/okhttp | 569 | 131K |
| androidx | github.com/androidx/androidx | 1,271 | 267K |

> To add a new fixture, edit the `FIXTURES` array in `scripts/setup-fixtures.sh`.

## Parity Goal

Full violation parity with JVM under `ktlint_official` (default) code style.
Current status tracked in `task_plan.md` → Known Parity Gaps.
