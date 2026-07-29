#!/usr/bin/env bash

source ./lib.sh

function greet() {
  echo "hi $1"
}

draw() {
  if [ -z "$1" ]; then
    greet "anonymous"
  else
    greet "$1"
  fi
}

draw "world"
