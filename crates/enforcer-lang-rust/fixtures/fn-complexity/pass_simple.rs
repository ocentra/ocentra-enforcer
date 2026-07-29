// PASS fixture for RUST-FN-COMPLEXITY: low branch count, guard-claused.
fn classify(x: i32) -> i32 {
    if x < 0 {
        return -1;
    }
    if x == 0 {
        return 0;
    }
    x
}
