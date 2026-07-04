// PASS fixture for RUST-BORROW-1.1: read-only param borrowed as `&str`.
fn greet(s: &str) -> String {
    format!("hello, {s}")
}
