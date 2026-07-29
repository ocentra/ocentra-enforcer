module Widget where

import Data.List (sort)
import qualified Data.Map as Map

data Shape = Circle Double | Rectangle Double Double

helper :: Int -> Int
helper x = x + 1

area :: Shape -> Double
area (Circle r) = pi * r * r
area (Rectangle w h) = w * h

draw :: Shape -> String
draw s =
  if area s > 0
    then "visible " ++ show (helper 3)
    else "hidden"
