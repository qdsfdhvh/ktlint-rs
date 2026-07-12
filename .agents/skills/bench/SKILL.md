# bench — Performance benchmark & table generator

Runs ktlint-rs vs ktlint JVM on real-world Kotlin projects (submodules) and
generates the Performance table for `README.md`.

## Usage

```bash
# Release mode (for final numbers):
./scripts/bench.sh --release

# Debug mode (faster build, for verifying):
./scripts/bench.sh
```

Output format:

```
Project                            Files     Lines (rs)  Lines (JVM)  Violations (rs)  Violations (JVM)  Time (rs)  Time (JVM)
-------------------------------------------------------------------------------------------------------------------
nowinandroid                        350       31,021       31,021         8,622           1,038             0.58s      10.1s
compose-samples (6 apps)            380       46,586       46,586         8,458              13             0.61s      11.3s
okhttp                              569      131,098      131,098        31,531              18             0.87s      19.6s
androidx (26 modules)              1271      266,549      266,549        72,558          33,731             0.86s      21.9s
```

## Prerequisites

- `ktlint` JVM CLI installed (`brew install ktlint`)
- Git submodules initialized: `git submodule update --init`
- For AndroidX: only 26 selected first-level modules are benchmarked
  (activity, fragment, compose-runtime, etc. — see `ANDROIDX_DIRS` in script)

## How the table is consumed

After running the script, copy the data rows into `README.md` under the
`## Performance` section. The table format uses split columns (`rs / JVM`)
for Lines, Violations, and Time to enable direct side-by-side comparison.

The long-term goal is full parity: ktlint-rs and ktlint JVM should report
identical Violations under the `android_studio` code style profile.
