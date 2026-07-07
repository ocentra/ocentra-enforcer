local json = require("json")

local Widget = {}

function Widget.new(name)
  local self = { name = name }
  return self
end

function Widget:draw()
  if self.name == "" then
    return "unnamed"
  end
  return helper(self.name)
end

function helper(label)
  return json.encode(label)
end

local w = Widget.new("world")
w:draw()
