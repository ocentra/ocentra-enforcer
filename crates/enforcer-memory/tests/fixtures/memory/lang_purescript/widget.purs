module Main where

import Prelude
import Effect (Effect)
import Effect.Console (log)

add :: Int -> Int -> Int
add a b = a + b

data Color = Red | Green | Blue

class Shape a where
  area :: a -> Number

main :: Effect Unit
main = do
  log (show (add 1 2))
