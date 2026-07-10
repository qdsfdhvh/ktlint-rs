package com.example.api

import com.example.model.User
import com.example.service.UserService
import java.util.*

class UserController(
private val service: UserService
) {
fun createUser(request: CreateUserRequest): User {
val user = User(
id = UUID.randomUUID().toString(),
name = request.name,
email = request.email
)
return service.save(user)
}

fun getUser(id: String): User?{
return service.findById(id)
}

fun listUsers(): List < User>{
return service.findAll()
}
}

data class CreateUserRequest(
val name: String,
val email: String
)
