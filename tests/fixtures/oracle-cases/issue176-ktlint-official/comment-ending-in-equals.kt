package com.example

class Hpack {
    fun readIndexedHeaderFieldInsidiousIndex() {
        bytesIn.writeByte(0xff) // == Indexed - Add ==
        bytesIn.write("8080808008".decodeHex()) // idx = -2147483521
        assertFailsWith<IOException> {
            hpackReader.readHeaders()
        }.also { expected ->
            assertThat(expected.message).isEqualTo("HPACK integer overflow")
        }
    }
}
