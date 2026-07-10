package com.example

import com.example.api.UserController
import com.example.model.User
import com.example.service.UserService
import org.junit.Test

class UserControllerTest {
@Test
fun `test create user`() {
val service = UserService()
val controller = UserController(service)
val request = com.example.api.CreateUserRequest("test","test@test.com")
val user = controller.createUser(request)
assert(user.name=="test")
assert(user.email=="test@test.com")
}

@Test
fun `test get user`() {
val service = UserService()
val controller = UserController(service)
controller.createUser(com.example.api.CreateUserRequest("test","test@test.com"))
val user = controller.getUser("non-existent")
assert(user== null)
}
}
