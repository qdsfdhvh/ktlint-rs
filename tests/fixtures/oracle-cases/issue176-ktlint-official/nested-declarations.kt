package com.example

fun outer() {
    fun inner(
        a: Int,
    ) {
        fun innermost(
            b: Int,
        ): String {
            return "$b"
        }
    }
}
