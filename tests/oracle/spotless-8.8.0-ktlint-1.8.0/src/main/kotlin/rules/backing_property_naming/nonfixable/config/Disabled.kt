package rules.backing_property_naming.nonfixable.config

class Disabled {
    val _items = mutableListOf<String>()

    val items: List<String>
        get() = _items
}
