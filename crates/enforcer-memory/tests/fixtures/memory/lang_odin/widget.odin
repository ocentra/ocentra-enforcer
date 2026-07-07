package widget

import "core:fmt"

Animal :: struct {
	name: string,
}

Dog :: struct {
	using animal: Animal,
	breed:        string,
}

Color :: enum {
	Red,
	Green,
}

draw :: proc(d: ^Dog) {
	helper(1, 2)
	obj.render()
}

render :: proc() {
	if true {
		helper()
	}
	for i := 0; i < 10; i += 1 {
		helper(i)
	}
	switch 1 {
	case 1:
		helper()
	}
}
