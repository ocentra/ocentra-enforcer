// PASS fixture for RUST-FN-MAX-PARAMS: params bundled into one input
// struct, function signature stays at 1 parameter.
struct FooInput {
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    e: i32,
    f: i32,
}

fn build(input: FooInput) -> i32 {
    input.a + input.b + input.c + input.d + input.e + input.f
}
