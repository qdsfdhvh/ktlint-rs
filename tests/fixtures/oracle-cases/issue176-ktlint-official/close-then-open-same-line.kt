package com.example

class Adapter {
    fun connectFailed(
        call: Call,
        ioe: IOException,
    ) = onEvent(
        ConnectFailed(
            System.nanoTime(),
            call,
            ioe,
        ),
    )

    fun connectionAcquired(
        call: Call,
    ) {
        doWork(call)
    }
}
