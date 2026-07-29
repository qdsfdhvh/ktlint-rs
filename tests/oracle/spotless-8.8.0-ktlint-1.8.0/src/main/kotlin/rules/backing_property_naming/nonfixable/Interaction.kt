package rules.backing_property_naming.nonfixable

class Interaction {
    private val _orphan = "value"

    fun interaction() = _orphan
}
