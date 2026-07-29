function greet
    echo "hi" $argv[1]
    draw $argv[1]
end

function draw
    echo $argv[1]
end

greet world
