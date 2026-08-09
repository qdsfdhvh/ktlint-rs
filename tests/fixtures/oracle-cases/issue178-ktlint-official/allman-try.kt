package com.example

fun f() {
    try
    {
        g()
    }
    catch (e: Exception)
    {
        h()
    }
    finally
    {
        i()
    }
}
