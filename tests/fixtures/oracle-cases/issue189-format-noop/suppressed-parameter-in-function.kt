package com.example

private fun setupTLSServer(
    @Suppress("SameParameterValue") port: Int,
    module: suspend Application.() -> Unit,
): EmbeddedServer<*, *> {
    return embeddedServer(
        factory = Jetty,
        configure = {},
    )
}
