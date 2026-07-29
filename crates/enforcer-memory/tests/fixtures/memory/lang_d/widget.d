module myapp.widgets;

import std.stdio;

class Animal {
    string name;

    this(string name) {
        this.name = name;
    }

    void speak() {
        writeln(name);
    }
}

class Dog : Animal {
    override void speak() {
        writeln(name);
        helper();
    }
}

int helper() {
    return add(1, 2);
}

int add(int a, int b) {
    return a + b;
}

struct Point {
    int x;
    int y;
}

void main() {
    auto d = new Dog();
    d.speak();
    helper();
}
