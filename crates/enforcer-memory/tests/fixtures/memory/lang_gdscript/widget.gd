extends "res://base_widget.gd"
class_name Widget

signal drawn(label)

@export var label: String = "widget"
onready var helper = get_node("Helper")

const MAX_COUNT = 10

var count: int = 0

func _init():
	count = 0

func draw() -> String:
	return label

func increment(amount: int) -> int:
	if amount > 0:
		count += amount
	else:
		count += 1
	for i in range(amount):
		emit_signal("drawn", label)
	return count

func render() -> void:
	helper.register(self)
	self.draw()
	super.draw()
