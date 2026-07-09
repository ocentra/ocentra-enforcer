local record Widget
   name: string
end

function Widget.new(name: string): Widget
   local self: Widget = { name = name }
   if name == "" then
      helper("unnamed")
   end
   return self
end

local function helper(label: string)
   print(label)
end

local w = Widget.new("world")
helper(w.name)
