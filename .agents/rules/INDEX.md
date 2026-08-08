# Rules Index — STOP-and-Read Triggers

Lookup table from task fact-pattern → rule file. Read on demand; do not load
the whole directory.

| Task area | Rule |
|---|---|
| **Any git write** — commit, push, branch, merge, PR | `git-workflow/RULE.md` |
| **Installing/updating the local `ktlint-rs` binary** on PATH (`~/.cargo/bin`), or tagging/releasing | `local-install/RULE.md` |
| Changing rules / formatter behavior that must stay parity-clean | See `docs/DESIGN.md` § parity; the Spotless oracle differential must stay green (`scripts/spotless-differential.sh --offline`) |
| Performance-sensitive changes | perf gates: `scripts/perf-gates.sh` (<30MB / <50ms / <5ms / clean exit) |
| Corpus validation | mutation gates: `scripts/mutation-test.sh` (idempotence + convergence) |
