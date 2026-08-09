package com.example

fun f() {
    for (
    step in generateSequence(1) { it * 2 }
        .dropWhile { it < 64 }
        .takeWhile { it <= 8192 }
    ) {
        bb.clear()
    }
}
