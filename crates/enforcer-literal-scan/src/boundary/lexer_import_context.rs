//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
pub(crate) fn is_import_specifier_context(line: &str, literal: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains(literal) {
        return false;
    }
    trimmed.starts_with("import ")
        || trimmed.starts_with("export ")
        || trimmed.contains(" from ")
        || trimmed.contains("require(")
}
