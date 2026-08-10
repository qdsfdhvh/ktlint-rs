package com.example

public sealed interface ExampleEvent {
    public data class ExampleNestedEvent(
        val alpha: String,
        val beta: String,
        val gammaxxxxxxxxxxxxxxxxxxxxxx: String,
    ) : ExampleEvent
}
