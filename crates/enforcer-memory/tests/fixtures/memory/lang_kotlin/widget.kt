package widget

import kotlin.math.max

interface Drawable {
    fun draw(): String
}

class Widget(val name: String) : Drawable {
    override fun draw(): String {
        if (name.isEmpty()) {
            return "unnamed"
        }
        return helper(name)
    }
}

fun helper(label: String): String {
    return label.uppercase()
}
