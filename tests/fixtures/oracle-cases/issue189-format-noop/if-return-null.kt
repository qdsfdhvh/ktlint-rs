package com.example

fun f(text: String): String? {
    if (text.any { it !in CHARS }) return null
    return text
}
