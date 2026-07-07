let add = fun x y => x + y in
let config = {
  name = "widget",
  count = add 1 2,
  enabled = if count > 0 then true else false,
} in
import "other.ncl"
