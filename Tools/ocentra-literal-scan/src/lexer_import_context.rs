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
