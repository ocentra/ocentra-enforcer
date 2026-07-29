// FAIL fixture for RUST-ERR-SENTINEL: sentinel return value instead of
// Result/Option.
fn find(items: &[i64], needle: i64) -> i64 {
    for (idx, item) in items.iter().enumerate() {
        if *item == needle {
            return idx as i64;
        }
    }
    -1
}
