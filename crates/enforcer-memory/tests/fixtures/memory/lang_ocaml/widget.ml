open Printf

type shape =
  | Circle of float
  | Rectangle of float * float

let helper x = x + 1

let area s =
  match s with
  | Circle r -> 3.14 *. r *. r
  | Rectangle (w, h) -> w *. h

let draw s =
  if area s > 0.0 then
    print_string "visible"
  else
    print_int (helper 3)
