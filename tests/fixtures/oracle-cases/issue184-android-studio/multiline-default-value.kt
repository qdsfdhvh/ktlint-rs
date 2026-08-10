package com.example

private fun newWebSocket(
    request: Request =
        Request
            .Builder()
            .url(
                "ws://example.com"
            ),
) {
    println(request)
}
