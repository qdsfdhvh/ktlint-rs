# Kataris Spotless oracle

Pinned behavioral oracle for Kataris:

- Spotless `8.8.0`
- ktlint `1.8.0`
- Gradle `9.6.1`
- Java `21`
- target `src/**/*.kt`
- target exclude `**/generated/**`

The checked-in `.editorconfig` is an exact copy of the Kataris root configuration whose SHA-256 is recorded in `oracle-manifest.json`.

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

`oracleFormat` mutates the fixture copy, so differential tests must run it in a temporary copy. Never point it at the Kataris checkout.

The Gradle wrapper properties are pinned, including the distribution checksum. The wrapper JAR is intentionally not duplicated here; CI should use its verified Gradle setup or generate the standard wrapper from Gradle 9.6.1.
