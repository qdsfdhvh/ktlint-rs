package com.example

private fun Int.toVeryLongDescriptiveEnumerationNameThatForcesTheReturnTypeOntoItsOwnLine():
    SomeQualifiedResultTypeName =
    when (this) {
        0 -> SomeQualifiedResultTypeName.ZERO
        else -> SomeQualifiedResultTypeName.OTHER
    }
