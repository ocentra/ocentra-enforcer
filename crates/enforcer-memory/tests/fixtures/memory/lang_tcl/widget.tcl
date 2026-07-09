proc greet {name} {
    puts "hi $name"
    draw $name
}

proc draw {name} {
    puts $name
}

namespace eval Widgets {
    proc make {} {
        puts hi
    }
}

greet world
