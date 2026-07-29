// PASS fixture for RUST-SAFETY-COMMENT: `unsafe` block carries a
// `// SAFETY:` comment immediately above it.
fn read_raw(ptr: *const i32) -> i32 {
    // SAFETY: caller guarantees `ptr` is valid and aligned for the
    // lifetime of this call.
    unsafe { *ptr }
}
