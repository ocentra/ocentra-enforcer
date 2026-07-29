function greet(name) {
    print "hi " name
    draw(name)
}

function draw(name) {
    print name
}

BEGIN {
    greet("world")
}
