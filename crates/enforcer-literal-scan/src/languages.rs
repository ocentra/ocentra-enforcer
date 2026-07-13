pub(crate) use crate::file_role::classify_file_role;
pub(crate) use crate::language_registry::detect_language;

/// Returns the registered language specifications exposed by this crate.
pub fn language_registry() -> Vec<crate::LanguageSpec> {
    crate::language_registry::language_registry()
}
