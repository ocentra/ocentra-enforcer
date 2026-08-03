//! Canonical parser identity registry for UL06.
//!
//! `registry/languages.json` is the sole reviewed input. `build.rs` validates
//! it and emits the records included below; this module contains no duplicate
//! language list and does not change parser behavior.

use crate::parsers::Language;
use enforcer_domain::language_types::{LanguageId, StructuralLanguageSupport};

/// One canonical parser identity and its structural parse disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageRecord {
    /// Stable one-based canonical identity.
    pub id: LanguageId,
    /// Existing parser enum variant preserved by this migration.
    pub parser: Language,
    /// Whether parser dispatch returns a structural result.
    pub structural: StructuralLanguageSupport,
}

include!(concat!(env!("OUT_DIR"), "/language_registry.rs"));

/// Return all canonical parser identities in stable registry order.
pub fn language_registry() -> &'static [LanguageRecord] {
    LANGUAGE_RECORDS
}

/// Find the canonical record for one existing parser variant.
pub fn record_for_parser(language: Language) -> Option<&'static LanguageRecord> {
    language_registry()
        .iter()
        .find(|record| record.parser == language)
}
