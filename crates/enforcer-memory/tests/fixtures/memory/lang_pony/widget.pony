use "collections"

class Animal
  var name: String

  new create(name': String) =>
    name = name'

  fun bark(): String =>
    "generic sound"

actor Dog is Animal
  new create(name': String) =>
    name = name'

  fun bark(): String =>
    add(1, 2)
    "woof"

primitive Helpers
  fun add(a: I64, b: I64): I64 =>
    a + b
