// FAIL fixture for RUST-FMT-CAPTURED-IDENT: positional format arg instead
// of an inline captured identifier.
fn describe(path: &str) -> String {
    format!("{}", path)
}
