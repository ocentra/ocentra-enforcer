// FAIL fixture for RUST-BORROW-1.1: read-only `String` param taken by
// value instead of by `&str`.
fn greet(s: String) -> String {
    format!("hello, {s}")
}
