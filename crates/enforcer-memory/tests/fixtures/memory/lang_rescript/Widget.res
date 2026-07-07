open Belt
include MyModule

exception WidgetError(string)

module Point = {
  type t = {x: int, y: int}
}

type color = Red | Green | Blue

let add = (a, b) => {
  a + b
}

let helper = (x) => {
  if x > 0 {
    Js.log("positive")
  } else {
    Js.log("non-positive")
  }
  switch x {
  | 0 => Js.log("zero")
  | _ => Js.log("other")
  }
  add(x, 1)
}

@react.component
let make = () => {
  helper(1)
  React.string("hi")
}
