package parity

@Target(AnnotationTarget.TYPE)
annotation class TypeMarker

enum class Compact { One, Two }

class Example<T : Any>(private val value : String) : Any() {
    fun compute(a: Int, b: Int): Int {
        val numbers = listOf(1, 2, 3)
        val range = 0..10
        val product = 2 * 3
        if (true) {
            println("if(x).. { }")
        }
// comment
        return a + b
    }

    fun lambda() {
        listOf(1).forEach { value -> println(value) }
    }

    val typed: List<@TypeMarker String> = emptyList()

    val raw = """
    keep } 
    catch and .. untouched
"""
}
