require "json"

class Animal
  def speak
    "..."
  end
end

class Widget < Animal
  @name : String

  def initialize(@name)
  end

  def draw
    if @name.empty?
      return "unnamed"
    end
    helper(@name)
  end
end

def helper(label)
  label.upcase
end
