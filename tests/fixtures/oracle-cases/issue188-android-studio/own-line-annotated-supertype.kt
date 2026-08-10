package com.example

public annotation class ExampleMarker(val name: String)

public sealed class ExampleVeryLongBaseTypeNameForOverflow {
    public data class ExampleAlpha(
        @ExampleMarker("p0")
        public val parameter0: String,
    ) : ExampleVeryLongBaseTypeNameForOverflow() {
        @ExampleMarker("kind")
        public val kind: String = "k"
    }

    public data class ExampleBeta(
        @ExampleMarker("p0")
        public val parameter0: String,
        @ExampleMarker("p1")
        public val parameter1: String,
    ) : ExampleVeryLongBaseTypeNameForOverflow() {
        @ExampleMarker("kind")
        public val kind: String = "k"
    }
}
