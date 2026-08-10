package com.example

fun f() {
    val server = embeddedServer(
        factory = Jetty,
        configure = {
            sslConnector(
                keyStore = testKeyStore,
                trustStore = testKeyStore,
            )
        },
    )
}
