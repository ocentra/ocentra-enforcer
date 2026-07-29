library(methods)

Animal <- function(name) {
  list(name = name)
}

helper <- function(x, y) {
  if (x > 0) {
    z <- x + y
  } else {
    z <- x - y
  }
  z
}

draw <- function(widget) {
  helper(1, 2)
  widget$render()
}

result <- draw(Animal("Widget"))
