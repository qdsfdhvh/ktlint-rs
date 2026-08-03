# Spotless oracle

Pinned behavioral oracle for a Spotless 8.8.0 + ktlint 1.8.0 pipeline:

- Spotless `8.8.0`
- ktlint `1.8.0`
- Gradle `9.6.1`
- Java `21`
- target `src/**/*.kt`
- target exclude `**/generated/**`

The checked-in `.editorconfig` is a synthetic Android Studio style reference profile (semantically equivalent to the pinned reference configuration) whose SHA-256 is recorded in `oracle-manifest.json`.

## Tasks

Run with a Gradle 9.6.1 executable or wrapper:

```sh
gradle --no-daemon --no-configuration-cache verifyOracleSnapshot
gradle --no-daemon --no-configuration-cache oracleCheck
gradle --no-daemon --no-configuration-cache oracleFormat
```

`oracleSnapshot` writes deterministic artifacts under `build/oracle/`:

- `diagnostics.json`
- `lint-exit-code.txt`
- `rule-inventory.json`
- `oracle-manifest.json`
- `effective-config.json`

`oracleFormat` mutates the fixture copy, so differential tests must run it in a temporary copy. Never point it at a real checkout.

The Gradle wrapper properties are pinned, including the distribution checksum. The wrapper JAR is intentionally not duplicated here; CI should use its verified Gradle setup or generate the standard wrapper from Gradle 9.6.1.


## Differential harness

```sh
# Fast committed-golden check (no JVM/Gradle invocation)
./scripts/spotless-differential.sh --offline

# Regenerate and compare all parity surfaces against the live pinned oracle
GRADLE=/path/to/gradle-9.6.1 ./scripts/spotless-differential.sh

# Prove mismatch detection and emit a minimized artifact set
GRADLE=/path/to/gradle-9.6.1 ./scripts/spotless-differential.sh --expect-mismatch
```

The harness compares discovery, effective configuration, normalized diagnostics (including autocorrectability), exit codes, and formatted bytes. Mismatch artifacts contain only files implicated by diagnostic or byte differences plus the normalized evidence and pinned metadata.
