module Widget where

open import Data.Nat

greet : Nat -> Nat
greet x = draw x

data Point : Set where
  point : Point
