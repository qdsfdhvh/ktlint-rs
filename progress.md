# Progress Log

## 2026-07-12 Session 3

### Done
- ✅ Implemented 3 missing JVM rules: `blank-line-before-declaration`, `no-blank-line-in-list`, `kdoc` (enhanced)
- ✅ Identified root cause: `editorconfig` crate strips non-standard keys
- ✅ Added `parse_ktlint_properties()` manual parsing and `find_editorconfig()`
- ✅ Created 3 new editorconfig test fixtures (profile, sections, mixed)
- ✅ Created `scripts/setup-fixtures.sh` for shallow-clone fixture management
- ✅ Removed `.gitmodules`, replaced with setup script
- ✅ Split planning into `task_plan.md` / `findings.md` / `progress.md`
- ✅ Updated README with latest benchmarks
- ✅ 185 tests all passing
- ✅ Full parity analysis: identified 5 gap categories with root causes

### Key Discoveries
- nowinandroid uses `ktlint_official`, NOT `android_studio` — earlier assumption was wrong
- Per-rule disable is PARSED but NOT WIRED to `RuleConfig` HashMap
- `blank-line-before-declaration` flags ALL declarations (1,240) while JVM only reports top-level (25)

### Next Session Priority
1. Wire per-rule `ktlint_standard_* = disabled` into `RuleConfig` HashMap
2. Fix indent rule to match JVM's context-sensitive logic
3. Tune `blank-line-before-declaration` to only flag top-level declarations

## 2026-07-12 Session 2

### Done
- ✅ Upgraded tree-sitter 0.24→0.26, kotlin-sg 0.4.0→0.4.1 (zero breaking changes)
- ✅ Added `rustfmt.toml` + global rustfmt application
- ✅ Integration test infrastructure (main.rs entry, real-project smoke tests)
- ✅ CI: added rustfmt check, submodule support
- ✅ Fixed `.editorconfig` absolute path resolution
- ✅ Updated Performance table with real benchmarks

## 2026-07-12 Session 1

### Done
- ✅ Created `scripts/bench.sh` for automated parity benchmarking
- ✅ Added per-rule breakdown + exit code verification to bench script
- ✅ Updated README Performance table with benchmark data
- ✅ Marked code_style as NOT WIRED in plan (pre-discovery phase)
