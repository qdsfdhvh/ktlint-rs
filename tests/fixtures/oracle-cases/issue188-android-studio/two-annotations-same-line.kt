package com.example

public annotation class ExampleA(val name: String)

public annotation class ExampleB

@ExampleA("x") @ExampleB
public val exampleValue: Int = 1
