package rules.backing_property_naming

class Positive {
    private val _items = mutableListOf<String>()

    val items: List<String>
        get() = _items
}
