module Widgets

using Base: show
import Helpers
export draw, area

abstract type Shape end

struct Dog <: Shape
    name::String
end

mutable struct Counter
    count::Int
end

function draw(w)
    helper(w)
    Base.show(w)
    return 1
end

function area(s::Shape)::Float64
    return 0.0
end

square(x) = x * x

function render()
    if square(2) > 0
        helper(1, 2)
    else
        helper(0, 0)
    end
    for i in 1:10
        helper(i)
    end
    while true
        break
    end
    try
        helper(1)
    catch e
        helper(2)
    end
end

end
