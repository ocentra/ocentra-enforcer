//! Closed identity values shared by canonical language projections.

use std::num::NonZeroU16;

/// Number of parser identities preserved by the UL06 canonical registry.
pub const PARSER_IDENTITY_COUNT: u16 = 160;

/// Stable, non-zero identity for a parser language.
///
/// BRAND-INVARIANT: the inner value is non-zero and within the canonical
/// parser identity range; callers obtain values through validated constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageId(NonZeroU16);

impl LanguageId {
    /// Validate and construct an identity from a registry index.
    pub fn try_new(index: u16) -> Result<Self, LanguageIdError> {
        if index == 0 {
            return Err(LanguageIdError::Zero);
        }
        if index > PARSER_IDENTITY_COUNT {
            return Err(LanguageIdError::OutOfRange);
        }
        let value = NonZeroU16::new(index).unwrap_or(NonZeroU16::MIN);
        Ok(Self(value))
    }

    /// Construct an identity from the canonical registry's one-based index.
    ///
    /// This constructor is used only by the statically reviewed registry
    /// projection. Invalid generated data fails immediately.
    pub const fn from_registry_index(index: u16) -> Self {
        assert!(
            index > 0 && index <= PARSER_IDENTITY_COUNT,
            "language registry index is outside the canonical identity range"
        );
        match NonZeroU16::new(index) {
            Some(value) => Self(value),
            None => Self(NonZeroU16::MIN),
        }
    }

    /// Return the stable registry index.
    pub const fn as_u16(self) -> u16 {
        self.0.get()
    }
}

/// Construction failure for a canonical language identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LanguageIdError {
    /// Zero cannot identify a language.
    #[error("language identity index must be non-zero")]
    Zero,
    /// The index is outside the reviewed parser identity range.
    #[error("language identity index is out of range")]
    OutOfRange,
}

/// Typed structural parse disposition for one canonical language identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralLanguageSupport {
    /// The parser dispatch can produce a structural file result.
    ParseFile,
    /// The parser dispatch intentionally has no structural extractor.
    NoParseFile,
}
