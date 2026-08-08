# Rules Index — STOP-and-Read Triggers

Lookup table from task fact-pattern → rule file. Read on demand; do not load
the whole directory.

| Task area | Rule |
|---|---|
| **Any git write** — commit, push, branch, merge, PR | `git-workflow/RULE.md` |
| **Installing/updating the local `ktlint-rs` binary** on PATH (`~/.cargo/bin`), or tagging/releasing | `local-install/RULE.md` |
| **Implementing/changing a rule or formatter behavior** — must stay parity-clean (oracle differential green, consumer corpus 0 violations) | `parity/RULE.md` |
| **Performance-sensitive changes** — parsing, rule engine, formatter, cache | `performance/RULE.md` (`scripts/perf-gates.sh`: <30MB / <50ms / <5ms / clean exit) |
| **Any task touching a consumer project** (kataris-app, ktor, …) — validation only, never edits | `scope/RULE.md` |
| Corpus validation | mutation gates: `scripts/mutation-test.sh` (idempotence + convergence) |
