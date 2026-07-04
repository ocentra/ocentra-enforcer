// PASS fixture for RUST-CAST-NO-AS-LOSSY: fallible conversion via
// `try_from`/`?` instead of a lossy `as` cast.
fn truncate(x: i64) -> Result<u8, std::num::TryFromIntError> {
    u8::try_from(x)
}
