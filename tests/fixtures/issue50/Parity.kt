annotation class Provides

data class Result(val first: String, val second: String)

class Module {
    @Provides
    fun provideOne(firstDependency: String, secondDependency: String): Result = Result(
        first = firstDependency,
        second = secondDependency
    )

    @Provides
    fun provideTwo(firstDependency: String, secondDependency: String): Result = Result(
        first = firstDependency,
        second = secondDependency
    )
}
