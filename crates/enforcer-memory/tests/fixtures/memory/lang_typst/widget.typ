#import "widget-lib.typ": draw_helper

#let helper(label) = {
  if label == "" {
    "unnamed"
  } else {
    label
  }
}

#let draw(label) = {
  helper(label)
}
