package com.example

public data class ExampleHolder(val items: List<String>)

private fun ExampleHolder.replaceTheFirstMatchingEntryWithAFreshlyComputedValue(
    computeReplacementValue: (String) -> String,
): ExampleHolder {
    val index = items.indexOfFirst { it.isNotBlank() }
    if (index >= 0) return copy(items = items.toMutableList().also { it[index] = "x" })
    return this
}
