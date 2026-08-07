# App Integration Guide (#89 / #90)

How to shadow and then cut over a Gradle Kotlin Multiplatform app (e.g.
kataris-app) from the Spotless + ktlint 1.8 plugin to `ktlint-rs` as the
authoritative check/format tool — reversibly.

## Phase 7 — Shadow (issue #89)

Add reversible root-project tasks with the same target/exclude scope as
the existing Spotless convention. Spotless remains authoritative until
the parity gates pass.

```kotlin
// build.gradle.kts (root) — or a build-logic convention applied once.
val ktlintRsArgs = listOf(
    "--ruleset", "ktlint",
    "--include", "**/src/**/*.kt",
    "--exclude", "**/generated/**", "--exclude", "**/build/**",
    "--exclude", "**/expected/**",
)
tasks.register<Exec>("ktlintRsCheck") {
    group = "verification"
    description = "Run ktlint-rs check (spotless-equivalent scope)"
    val bin = System.getenv("KTLINT_RS_BIN") ?: "ktlint-rs"
    commandLine(listOf(bin) + ktlintRsArgs + listOf("--strict", "."))
}
tasks.register<Exec>("ktlintRsFormat") {
    group = "formatting"
    description = "Run ktlint-rs --format (spotlessApply equivalent)"
    val bin = System.getenv("KTLINT_RS_BIN") ?: "ktlint-rs"
    commandLine(listOf(bin) + ktlintRsArgs + listOf("--format", "."))
}
```

CI: run `spotlessCheck` and `ktlintRsCheck` side by side. Both must agree
on every Kotlin-changing PR; any mismatch is a ktlint-rs parity bug.

Rollback: delete the two tasks; Spotless was never touched.

## Phase 8 — Cut over (issue #90)

Once shadow gates are green:

1. Replace the `kataris.spotless` convention's ktlint engine with
   `ktlintRsCheck`/`ktlintRsFormat`, or point `spotlessKotlin` at a
   `ktlint-rs` executable — do **not** keep two engines formatting.
2. Keep the pinned manual oracle (real ktlint 1.8 CLI) available during
   stabilization: `./scripts/spotless-differential.sh --offline` runs it.
3. Remove the Spotless plugin dependency only after every parity/shadow
   gate passes.

Rollback: restore the Spotless convention from git; `spotlessApply`
rewrites files to the ktlint 1.8 canonical form (ktlint-rs is
byte-compatible with it on the App corpus — 0 check violations, 0
`--format` changes as of 0.1.7).

## Verification tooling (ktlint-rs side, already in CI)

- `scripts/perf-gates.sh` — binary size, cold startup, per-file time,
  clean-exit gates (#88).
- `scripts/mutation-test.sh <bin> <corpus> <n>` — fixed-seed mutation
  corpus: idempotence + convergence (#87).
- `scripts/spotless-differential.sh --offline` — byte/diagnostic parity
  against the pinned Spotless 8.8.0 + ktlint 1.8.0 oracle.
- kataris-app corpus is re-verified on every change: `check` 0
  violations, `--format` changes 0 files.

## Scope notes

The kataris-app repository itself is owned by the app team; this guide
documents the mechanics. The tasks/CI wiring above are applied by the
app maintainers when the shadow phase starts.
