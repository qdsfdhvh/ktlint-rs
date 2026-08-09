package com.example

fun f(
    a: Int = run {
        1
    },
    b: Int,
) {
    println(b)
}
