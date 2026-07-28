# Spotless parity fixture

`expected.kt` was generated from `input.kt` with:

- Spotless `8.8.0`
- ktlint `1.8.0`
- Gradle `9.4.1`
- Kataris `.editorconfig` (`android_studio`, 4 spaces, 120 columns)

Oracle command (in an isolated temporary Gradle project):

```sh
./gradlew --no-daemon --no-configuration-cache spotlessApply
```

The fixture intentionally includes strings/comments containing formatting tokens and a generated-source exclusion companion in the external differential harness. Kotlin files are compared byte-for-byte without whitespace normalization.
