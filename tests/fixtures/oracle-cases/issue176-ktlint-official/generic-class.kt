package com.example

class Box<T>(
    val value: T,
) : Base() {
    fun get(): T {
        return value
    }
}
