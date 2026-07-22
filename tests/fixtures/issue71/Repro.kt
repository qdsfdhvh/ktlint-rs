enum class AuthAction {
    Email,
}

class Repro {
    fun isPositive(count: Int): Boolean = count > 0

    fun classify(count: Int): String = when (count) {
        0 -> "zero"
        else -> "other"
    }

    fun catchFailure() {
        try {
            error("boom")
        } catch (error: IllegalStateException) {
            println(error)
        }
    }

    fun render(content: () -> Unit) {
        content()

        val itemHeight = 56
        println(itemHeight)
    }

    fun callback(): (Any?) -> Unit = ::println
}

fun DebugMenuEntry() = Unit

fun blankString(): String = "   "

/** Valid KDoc on a private declaration. */
private fun helper() = Unit

// Numeric comparison: 0 < 1.
