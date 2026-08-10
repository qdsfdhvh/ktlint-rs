package com.example

public annotation class ExampleMarker(val name: String)

public data class ExampleShort(
    @ExampleMarker("id")
    public val id: String,
)
