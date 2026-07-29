// FAIL fixture for RUST-SAFETY-COMMENT: `unsafe` block with no `// SAFETY:`
// comment immediately above it.
fn read_raw(ptr: *const i32) -> i32 {
    unsafe { *ptr }
}
