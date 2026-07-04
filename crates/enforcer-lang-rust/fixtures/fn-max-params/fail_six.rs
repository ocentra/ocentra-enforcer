// FAIL fixture for RUST-FN-MAX-PARAMS: function with 6 parameters (max 5).
fn build(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
    a + b + c + d + e + f
}
