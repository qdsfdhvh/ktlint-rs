package com.example.service

import com.example.model.User

class UserService {
private val users = mutableListOf < User > ()

fun save(user: User): User {
users.add(user)
return user
}

fun findById(id: String): User?{
return users.find {it.id== id}
}

fun findAll(): List < User>{
return users.toList()
}
}
