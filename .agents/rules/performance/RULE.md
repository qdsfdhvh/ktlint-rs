# Performance Rules

Read before changes that touch the hot path (parsing, rules, formatter,
cache).

## Hard constraints (enforced by `scripts/perf-gates.sh` in CI)

- Release binary < 30MB.
- Cold startup < 50ms (after `.cache` warm).
- Per-file engine lint < 5ms average.
- Clean exit — no daemon, no residual thread/CPU activity after linting.

## Rule authoring

- Each rule's `check()` must be O(n) and side-effect free.
- Free memory immediately after linting; no caching of file contents.
- No background threads / rayon pool that outlives the run.

## When to run

Any PR that changes parsing, the rule engine, or the formatter: run
`scripts/perf-gates.sh target/release/ktlint-rs` locally before pushing.
