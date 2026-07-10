package com.example

import com.example.api.UserController
import com.example.api.CreateUserRequest
import com.example.service.UserService

fun main() {
val service = UserService()
val controller = UserController(service)

val request = CreateUserRequest(
name ="Alice",
email ="alice@example.com"
)

val user = controller.createUser(request)
println("Created user: ${user.name}")

val found = controller.getUser(user.id)
println("Found user: ${found?.name}")

val allUsers = controller.listUsers()
println("All users: ${allUsers.size}")
}
