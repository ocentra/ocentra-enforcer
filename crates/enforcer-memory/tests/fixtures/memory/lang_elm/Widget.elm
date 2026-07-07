module Widget exposing (area)

import List exposing (map)
import Html.Attributes as Attr

type Shape
    = Circle Float
    | Square Float

area : Shape -> Float
area shape =
    case shape of
        Circle r ->
            helper r

        Square s ->
            s * s

helper : Float -> Float
helper r =
    3.14 * r * r
