package rules.backing_property_naming.nonfixable

class Negative {
    val _items = mutableListOf<String>()

    val items: List<String>
        get() = _items
}
