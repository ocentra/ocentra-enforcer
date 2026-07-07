require 'json'
require_relative './helper'

class Animal
  def speak
    puts "..."
  end
end

class Widget < Animal
  def initialize(name)
    @name = name
  end

  def draw
    if @name.empty?
      return "unnamed"
    end
    helper(@name)
  end

  def self.create
    Widget.new("default")
  end
end

def helper(label)
  label.upcase
end
