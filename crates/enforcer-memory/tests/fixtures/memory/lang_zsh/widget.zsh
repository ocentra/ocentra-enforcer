function greet() {
    echo "hi" $1
    draw $1
}

draw() {
    echo $1
}

greet world
