use fmt;

type animal = struct {
	name: str,
};

fn add(a: int, b: int) int = {
	return a + b;
};

export fn main() void = {
	let x = add(1, 2);
	if (x > 0) {
		fmt::println("positive")!;
	};
};
