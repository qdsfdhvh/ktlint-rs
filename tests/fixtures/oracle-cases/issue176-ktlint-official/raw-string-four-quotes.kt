package com.example

fun f() {
    assertEquals(""""1""kotlin"""", contextualResult, UserDataSerializer)
    assertEquals(
        """{"id":1,"name":"kotlin"}""",
        simpleResult,
        defaultSerializationFormat,
        UserData.serializer()
    )
}
