package com.example

public annotation class ExampleMarker(val name: String)

public data class ExampleSame(
    @ExampleMarker("id") public val id: String,
)
