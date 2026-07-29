// PASS fixture for RUST-ERR-SENTINEL: `Option<T>` instead of a sentinel
// value.
fn find(items: &[i64], needle: i64) -> Option<usize> {
    items.iter().position(|item| *item == needle)
}
