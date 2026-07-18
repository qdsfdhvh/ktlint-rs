# kotlinc Type Extractor Plugin

Kotlin compiler plugin for **ktlint-rs Phase 13** — full type resolution via `kotlinc`.

## Usage

```bash
# Run as a Kotlin script (requires kotlinc on PATH):
kotlinc -script TypeExtractor.kts -- /path/to/source.kt

# Output JSON (stdout):
# {
#   "version": 1,
#   "declarations": { "x": {"type": "String", "nullable": false, "line": 3} },
#   "return_types": { "foo": "Int" }
# }
```

## Integration with ktlint-rs

```bash
ktlint-rs --kotlinc-path /usr/local/bin/kotlinc --ruleset detekt
```

## Dependencies

- Kotlin compiler (`kotlinc`) 1.9+
- Kotlin standard library
- IntelliJ Platform core (bundled with kotlinc)

## Status

✅ Plugin functional — produces valid JSON for basic types
🟡 Requires kotlin-analysis API for full binding context resolution
⬜ Support for generics, type parameters, extension receivers
