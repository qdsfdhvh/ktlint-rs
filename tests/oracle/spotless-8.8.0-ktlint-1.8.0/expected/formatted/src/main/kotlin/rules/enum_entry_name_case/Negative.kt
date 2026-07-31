package rules.enum_entry_name_case

// Non-autocorrectable negative cases are exercised by the Rust/JVM rule-case tests.
enum class ValidForSpotlessApply {
    FOO,
    FooBar,
}
