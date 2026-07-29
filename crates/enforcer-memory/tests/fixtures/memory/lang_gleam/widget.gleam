import gleam/io

pub type Shape {
  Circle(radius: Float)
  Square(side: Float)
}

pub fn helper(label: String) -> Nil {
  io.println(label)
}

pub fn draw(name: String) -> Nil {
  case name {
    "" -> io.println("unnamed")
    _ -> helper(name)
  }
}
