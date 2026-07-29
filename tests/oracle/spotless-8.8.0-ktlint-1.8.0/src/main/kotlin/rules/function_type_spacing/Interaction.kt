package rules.function_type_spacing

suspend fun consume(callback: suspend() -> Unit) = callback()
fun String ?.trimmed() = trim()
