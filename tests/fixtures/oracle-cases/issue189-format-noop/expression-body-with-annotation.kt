package com.example

class Trust {
    @Suppress("unused")
    fun parametersToString(): String =
        if (parameters.isEmpty()) "" else "; ${parameters.joinToString(";")}"
}
