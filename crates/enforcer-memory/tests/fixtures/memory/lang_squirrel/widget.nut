class Animal {
    name = "";

    constructor(n) {
        name = n;
    }

    function speak() {
        return "...";
    }
}

class Dog extends Animal {
    function speak() {
        base.speak();
        return "Woof";
    }

    function fetch(item) {
        print(item);
        return item;
    }
}

enum Color {
    Red,
    Green,
    Blue = 3
}

function makeDog(name) {
    local d = Dog(name);
    d.speak();
    d.fetch("ball");
    return d;
}

function run() {
    local dog = makeDog("Rex");
    if (dog != null) {
        print("has dog");
    } else {
        print("no dog");
    }
    switch (dog.speak()) {
        case "Woof":
            print("barked");
            break;
        default:
            print("silent");
    }
    local i = 0;
    while (i < 3) {
        i += 1;
    }
}
