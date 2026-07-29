/// Returns the registered language specifications exposed by this crate.
pub fn language_registry() -> Vec<crate::LanguageSpec> {
    crate::language_registry::language_registry()
}
