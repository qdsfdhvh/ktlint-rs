package com.example

/**
 * Example lifted from a JDK KDoc:
 *
 * ```java
 *    public List<X509Certificate> checkServerTrusted(
 *        X509Certificate[] chain, String authType) {
 *    }
 * ```
 */
class Trust {
    fun sslSocketFactory(
        factory: SSLSocketFactory,
    ) = apply {
        if (factory != this.factory) {
            this.factory = factory
        }
    }
}
