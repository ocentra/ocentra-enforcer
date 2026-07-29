-module(widget).
-export([area/1, helper/1, draw/1]).
-import(lists, [sort/1]).

-type shape() :: {circle, float()} | {rectangle, float(), float()}.

helper(X) ->
    X + 1.

area({circle, R}) ->
    3.14 * R * R;
area({rectangle, W, H}) ->
    W * H.

draw(S) ->
    if
        area(S) > 0 -> io:format("visible~n");
        true -> io:format("hidden~n")
    end,
    helper(3).
