// FAIL fixture for RUST-CAST-NO-AS-LOSSY: narrowing numeric `as` cast.
fn truncate(x: i64) -> u8 {
    x as u8
}
