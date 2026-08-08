# Parity Rules

Read before implementing or changing any rule/formatter behavior.

## Hard gates (must all pass)

- **Spotless oracle differential**: `scripts/spotless-differential.sh --offline`
  must print "all match the Spotless oracle". Re-generate parity artifacts
  (`scripts/generate_rule_plan.py --binary target/release/ktlint-rs`) after
  registering a rule.
- **Consumer corpus**: kataris-app (and ktor fixtures) must stay at **0
  violations** (besides genuine ones that real ktlint also reports) and
  `--format` must change **0 files**.
- **Unit + integration**: `cargo test` all green.

## New rule / fail-closed rule discipline

- Replace a fail-closed placeholder with a **CST-aware** implementation
  (tree-sitter), never a line-scan heuristic.
- Verify against the real ktlint 1.8 CLI: line:col identical on samples.
- Regress against kataris-app — a single false positive is a failure.
- When the oracle's fixture corpus conflicts with a desired behavior (e.g.
  #127 signature collapse vs parameter-list-wrapping), **the oracle wins** —
  keep the differential green and document the conservative gap.

## Verification tooling

- `scripts/mutation-test.sh` — idempotence + convergence on mutated corpus.
- `scripts/perf-gates.sh` — runtime constraints (see performance/RULE.md).
