package com.example.util

import java.util.regex.Pattern

object EmailValidator {
private val EMAIL_PATTERN = Pattern.compile(
"^[A-Za-z0-9+_.-]+@[A-Za-z0-9.-]+\\.[A-Za-z] {2,}$"
)

fun isValid(email: String): Boolean {
return EMAIL_PATTERN.matcher(email).matches()
}

fun validateAll(emails: List < String>): List < String>{
return emails.filter {isValid(it)}
}
}

object Logger {
enum class Level {
DEBUG, INFO, WARN, ERROR
}

private var currentLevel = Level.INFO

fun log(level: Level,message: String) {
if(level.ordinal>= currentLevel.ordinal) {
println("[${level.name}] $message")
}
}

fun debug(message: String) = log(Level.DEBUG,message)
fun info(message: String) = log(Level.INFO,message)
fun warn(message: String) = log(Level.WARN,message)
fun error(message: String) = log(Level.ERROR,message)
}
