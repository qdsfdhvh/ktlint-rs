# Git Workflow Rules

Read before any commit, push, branch, merge, or PR.

## Hard rules

- ❌ NEVER push directly to `main` — always branch (`feat/...`, `fix/...`,
  `docs/...`, `ci/...`, `chore/...`) and open a PR.
- ❌ NEVER force-push or `--amend` pushed commits.
- ❌ NEVER commit machine-specific paths or unrelated file changes (e.g. a
  consumer project's files) into this repo.
- ✅ Conventional commits: `feat:`, `fix:`, `docs:`, `ci:`, `refactor:`,
  `test:`, `chore:`.
- ✅ Docs-only changes (`**.md`) skip CI via `paths-ignore` — but still go
  through a branch + PR.
- ✅ Before merging a PR: all required checks must pass (at least fmt, tests,
  spotless differential; perf/mutation gates when code changes).
- ✅ After merging, sync local `main` from `origin/main`.

## Version bumps

See `local-install/RULE.md` § Release procedure — never bump the version in
the same PR as feature work.
