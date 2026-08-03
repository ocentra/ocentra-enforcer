//! Canonical parser identity registry for UL06.
//!
//! `registry/languages.json` is the sole reviewed input. `build.rs` validates
//! it and emits the records included below; this module contains no duplicate
//! language list and does not change parser behavior.

use crate::parsers::Language;
use enforcer_domain::language_types::{
    CollisionResolution, DetectionMatcher, DetectionMatcherKind, DetectionPrecedenceProjection,
    DetectionPrecedenceTieBreak, LanguageId, LiteralDisposition, LiteralProjection,
    LiteralProjectionDisposition, LiteralReference, MatcherWinner, StructuralLanguageSupport,
};
use std::fmt;
use std::num::NonZeroU16;

/// Closed canonical parser name emitted by the reviewed registry.
/// BRAND-INVARIANT: the value is created only from validated static registry data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalLanguageName(&'static str);

impl CanonicalLanguageName {
    /// Return the validated canonical name as a typed display value.
    #[must_use]
    pub const fn value(self) -> Self {
        self
    }
}

impl fmt::Display for CanonicalLanguageName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// One canonical parser identity and its structural parse disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageRecord {
    /// Stable one-based canonical identity.
    id: LanguageId,
    /// Existing parser enum variant preserved by this migration.
    parser: Language,
    /// Whether parser dispatch returns a structural result.
    structural: StructuralLanguageSupport,
    /// Stable canonical name used by identity-preserving consumers.
    /// BRAND-INVARIANT: this value is emitted only from the validated reviewed
    /// manifest and has static lifetime in the generated registry.
    canonical_name: &'static str,
    /// Globally unique aliases, when present.
    /// BRAND-INVARIANT: aliases are case-folded and validated globally before
    /// the generated registry is compiled.
    aliases: &'static [&'static str],
    /// Detection metadata owned by the canonical registry.
    matchers: &'static [DetectionMatcher],
    /// Typed literal support disposition.
    literal_disposition: LiteralDisposition,
}

const fn require_nonzero(index: Option<NonZeroU16>) -> NonZeroU16 {
    match index {
        Some(value) => value,
        None => loop {},
    }
}

const fn language_id_from_registry_index(index: NonZeroU16) -> LanguageId {
    LanguageId::from_registry_index(index)
}

impl LanguageRecord {
    /// Return the stable one-based canonical identity.
    #[must_use]
    pub const fn id(&self) -> LanguageId {
        self.id
    }

    /// Return the parser identity preserved by this record.
    pub const fn parser(&self) -> Language {
        self.parser
    }

    /// Return the structural parse disposition.
    pub const fn structural(&self) -> StructuralLanguageSupport {
        self.structural
    }

    /// Return the validated canonical name from the reviewed registry.
    #[must_use]
    pub const fn canonical_name(&self) -> CanonicalLanguageName {
        CanonicalLanguageName(self.canonical_name)
    }

    /// Return canonical detection matchers.
    pub const fn matchers(&self) -> &'static [DetectionMatcher] {
        self.matchers
    }

    /// Return the typed literal support disposition.
    pub const fn literal_disposition(&self) -> LiteralDisposition {
        self.literal_disposition
    }
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
        .find(|record| record.parser() == language)
}

/// Return the complete literal-to-parser crosswalk.
pub fn literal_projections() -> &'static [LiteralProjection] {
    LITERAL_PROJECTIONS
}

/// Return explicit same-key matcher collision decisions.
pub fn collision_resolutions() -> &'static [CollisionResolution] {
    COLLISION_RESOLUTIONS
}

/// Return the reviewed matcher precedence policy without running detection.
pub fn detection_precedence() -> &'static DetectionPrecedenceProjection {
    &DETECTION_PRECEDENCE
}

#[cfg(test)]
mod tests {
    use super::language_registry;

    #[test]
    fn canonical_name_projection_is_non_empty() {
        assert!(language_registry().iter().all(|record| !record
            .canonical_name()
            .value()
            .to_string()
            .is_empty()));
    }
}
