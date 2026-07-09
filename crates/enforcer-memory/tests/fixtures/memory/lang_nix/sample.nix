{ pkgs, lib, ... }:
let
  addOne = x: x + 1;
in
{
  foo = addOne 41;
  bar = if addOne 1 then 1 else 2;
}
