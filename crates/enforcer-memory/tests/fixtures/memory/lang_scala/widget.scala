package com.example.widget

import scala.collection.mutable
import com.example.other.Foo

trait Drawable {
  def draw(): String
}

class Base(val id: Int)

class Widget(val name: String) extends Base(0) with Drawable {
  def draw(): String = {
    if (name.isEmpty) {
      "x"
    } else {
      for (i <- 0 until 3) {
        helper()
      }
      bar.render(name)
    }
  }
}

object Widget {
  def apply(name: String): Widget = new Widget(name)
}

def helper(): Unit = {}
