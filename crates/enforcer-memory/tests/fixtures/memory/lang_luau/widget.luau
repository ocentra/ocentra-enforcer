type Point = { x: number, y: number }

local function helper(label: string)
	print(label)
end

function Widget.new(name: string)
	local self = { name = name }
	if name == "" then
		helper("unnamed")
	end
	return self
end

local w = Widget.new("world")
w:draw()
