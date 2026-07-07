package com.example.widget

import com.example.other.Foo
import com.example.other.Bar

class Base {
    int id;
}

interface Drawable {
    String draw()
}

class Widget extends Base implements Drawable {
    String name;

    String draw() {
        if (name.isEmpty()) {
            return "x"
        }
        for (int i = 0; i < 3; i++) {
            helper()
        }
        return bar.render(name)
    }
}

enum Status {
    ACTIVE, INACTIVE
}

def helper() {
    println "helping"
}

def topLevel() {
    def w = new Widget()
    w.draw()
}
