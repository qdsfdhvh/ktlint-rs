package com.example

fun f() {
    val r = run {
        g(
            1,
            2,
        )
    }
    h(
        g(
            1,
        ),
        2,
    )
}
