package com.example

class C(
    val a: Int,
) {
    constructor(
        b: String,
    ) : this(1)

    fun g(
        x: Int = foo(1),
        y: Int,
    ) {
        println(y)
    }
}
