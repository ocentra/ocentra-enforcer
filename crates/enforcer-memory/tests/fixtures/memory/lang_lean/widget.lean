import Mathlib.Data.Nat.Basic

def helper (x : Nat) : Nat := x + 1

def area (shape : Nat) : Nat := helper shape

theorem area_thm (x : Nat) : helper x = x + 1 := rfl

structure Point where
  x : Nat
  y : Nat
