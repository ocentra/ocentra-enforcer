//! Closed identity values shared by canonical language projections.

use std::num::NonZeroU16;

/// Number of parser identities preserved by the UL06 canonical registry.
pub const PARSER_IDENTITY_COUNT: u16 = 160;

/// Number of canonical parser identities that produce structural facts.
pub const STRUCTURAL_IDENTITY_COUNT: usize = 156;

/// Number of canonical identities intentionally without a structural parser.
pub const NO_PARSE_FILE_IDENTITY_COUNT: usize = 4;

/// Number of literal projection rows, including the explicit fallback row.
pub const LITERAL_PROJECTION_COUNT: usize = 68;

/// Number of named literal projection rows before the fallback row.
pub const NAMED_LITERAL_COUNT: usize = 67;

/// Number of canonical identities without a literal-row projection.
pub const NO_LITERAL_PARSER_IDENTITY_COUNT: usize = 85;

/// Stable, non-zero identity for a parser language.
///
/// BRAND-INVARIANT: the inner value is non-zero and within the canonical
/// parser identity range; callers obtain values through validated constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LanguageId(NonZeroU16);

/// Rejection returned when a registry identity is outside the reviewed range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvalidLanguageId;

impl LanguageId {
    /// Try to construct an identity from the canonical registry's one-based index.
    #[must_use]
    pub const fn try_from_registry_index(index: NonZeroU16) -> Result<Self, InvalidLanguageId> {
        if index.get() <= PARSER_IDENTITY_COUNT { Ok(Self(index)) } else { Err(InvalidLanguageId) }
    }

    /// Construct a checked identity from the canonical registry's one-based index.
    ///
    /// This const-safe path is used only by the statically reviewed registry
    /// projection. Invalid generated data fails at compile time rather than
    /// being replaced with another identity.
    pub const fn from_registry_index(index: NonZeroU16) -> Self {
        assert!(index.get() <= PARSER_IDENTITY_COUNT);
        Self(index)
    }
}

/// Typed structural parse disposition for one canonical language identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StructuralLanguageSupport {
    /// The parser dispatch can produce a structural file result.
    ParseFile,
    /// The parser dispatch intentionally has no structural extractor.
    NoParseFile,
}

/// Matcher kind used by canonical language detection metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectionMatcherKind {
    /// A file extension such as `rs`.
    Extension,
    /// A complete case-insensitive filename such as `Dockerfile`.
    ExactBasename,
    /// A compound suffix such as `.env.local`.
    CompoundSuffix,
}

/// One closed detection matcher owned by the canonical registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectionMatcher {
    /// An extension matcher.
    Extension(&'static str),
    /// An exact basename matcher.
    ExactBasename(&'static str),
    /// A compound suffix matcher.
    CompoundSuffix(&'static str),
}

/// Tie rule for matchers that share the same detection kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectionPrecedenceTieBreak {
    /// Prefer the longest compound suffix before a shorter suffix.
    LongestValue,
}

/// Reviewed matcher precedence metadata; this is policy data, not a detector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DetectionPrecedenceProjection {
    ordered_kinds: [DetectionMatcherKind; 3],
    same_kind_tie_break: DetectionPrecedenceTieBreak,
}

impl DetectionPrecedenceProjection {
    /// Construct the typed projection emitted from the reviewed manifest.
    pub const fn from_reviewed(ordered_kinds: [DetectionMatcherKind; 3], same_kind_tie_break: DetectionPrecedenceTieBreak) -> Self {
        Self { ordered_kinds, same_kind_tie_break }
    }

    /// Return the global matcher-kind order.
    pub const fn ordered_kinds(&self) -> &[DetectionMatcherKind; 3] {
        &self.ordered_kinds
    }

    /// Return the same-kind tie rule.
    pub const fn same_kind_tie_break(&self) -> DetectionPrecedenceTieBreak {
        self.same_kind_tie_break
    }
}

/// Typed disposition for a canonical identity's literal projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralDisposition {
    /// The identity participates in a named literal projection row.
    Registered { literal_name: &'static str },
    /// The identity is canonical but has no current literal-row support.
    Unsupported,
    /// The identity is intentionally non-structural and not applicable to literal routing.
    NotApplicable,
}

/// Typed target for a literal matcher winner or collision member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralReference {
    /// A canonical parser identity.
    ParserId(LanguageId),
    /// A supplemental literal identity without a parser identity.
    SupplementalLiteralName(&'static str),
    /// The explicit unknown fallback.
    Fallback,
}

/// Crosswalk disposition for one literal projection row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralProjectionDisposition {
    /// A row linked to one or more canonical parser identities.
    Registered,
    /// A named literal row with no canonical parser identity.
    LiteralOnly,
    /// The explicit unknown fallback row.
    Fallback,
}

/// Winner selection for one matcher key within a projection row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatcherWinner {
    /// One typed matcher key and its deterministic target.
    Key(&'static str, LiteralReference),
}

/// One machine-readable literal-to-parser crosswalk row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralProjection {
    /// Stable literal row, disposition, parser IDs, typed matchers, and winners.
    Row(
        &'static str,
        LiteralProjectionDisposition,
        &'static [LanguageId],
        &'static [DetectionMatcher],
        &'static [MatcherWinner],
    ),
}

/// One explicit same-key collision resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollisionResolution {
    /// Matcher kind, normalized key, members, and one winner.
    Group(DetectionMatcherKind, &'static str, &'static [LiteralReference], LiteralReference),
}
