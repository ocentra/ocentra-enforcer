//! Canonical pure values used by the structured rule registry.
//!
//! JSON catalog spelling belongs to `enforcer-rules::boundary`; these values
//! carry only validated domain meaning after the catalog boundary.

use crate::boundary::decode_error::DecodeError;
use crate::hashes::Sha256;
use crate::ids::RuleId;

/// Canonical outcome of comparing two versions of one rule record.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for VersionDriftOutcome."]
pub enum VersionDriftOutcome {
    Unchanged,
    ContentChangedVersionBumped,
    ContentChangedVersionNotBumped,
    VersionBumpedContentUnchanged,
    RuleIdMismatch,
}

/// Whether a rule comparison requires fail-closed handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionDriftStatus {
    Clean,
    Drifted,
}

/// Whether parity-bearing rule content changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleParityChange {
    Unchanged,
    Changed,
}

/// Result of a pure rule-analysis predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for RulePredicateResult."]
pub enum RulePredicateResult {
    Matched,
    NotMatched,
}

/// Cyclomatic complexity measured for one Rust function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for RustCyclomaticComplexity."]
#[doc = "BRAND-INVARIANT: represents a non-negative rule-analysis branch count."]
pub struct RustCyclomaticComplexity(u32);

impl RustCyclomaticComplexity {
    #[must_use]
    pub const fn from_count(value: u32) -> Self {
        Self(value)
    }

    pub fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

    pub fn increment_by(&mut self, amount: Self) {
        self.0 = self.0.saturating_add(amount.0);
    }
}

impl std::fmt::Display for RustCyclomaticComplexity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Number of explicit parameters in one Rust function signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "Canonical domain representation for RustExplicitParameterCount."]
#[doc = "BRAND-INVARIANT: represents a non-negative rule-analysis parameter count."]
pub struct RustExplicitParameterCount(usize);

impl RustExplicitParameterCount {
    #[must_use]
    pub fn from_parameters(parameters: impl IntoIterator) -> Self {
        Self(parameters.into_iter().count())
    }

    #[must_use]
    pub const fn from_count(value: usize) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for RustExplicitParameterCount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Nesting depth inside Rust test-only syntax scopes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for RustTestNestingDepth."]
#[doc = "BRAND-INVARIANT: represents a non-negative syntax traversal depth."]
pub struct RustTestNestingDepth(usize);

impl RustTestNestingDepth {
    pub fn enter(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

    pub fn exit(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    #[must_use]
    pub const fn state(self) -> RulePredicateResult {
        if self.0 == 0 {
            RulePredicateResult::NotMatched
        } else {
            RulePredicateResult::Matched
        }
    }
}

/// Cardinality of a canonical rule registry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
#[doc = "BRAND-INVARIANT: represents a non-negative in-memory rule-record cardinality."]
pub struct RuleRecordCount(usize);

impl RuleRecordCount {
    #[must_use]
    pub fn from_records(records: impl IntoIterator) -> Self {
        Self(records.into_iter().count())
    }
}

/// Whether a canonical rule registry contains records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleRegistryState {
    Empty,
    Populated,
}

use std::collections::BTreeMap;
use std::num::NonZeroU32;

macro_rules! rule_text {
    ($(#[$doc:meta])* $name:ident, $field:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        // BRAND-INVARIANT: this private text is created only by the checked
        // conversion below, which rejects empty and control-character values.
        pub struct $name(String);

        impl $name {
            /// View the validated value.
            #[must_use]
            #[doc = "The as_str operation for this canonical domain value."]
            pub fn as_str(&self) -> &str { &self.0 }

            /// Build diagnostic text with a stable non-empty fallback.
            #[must_use]
            pub fn from_diagnostic(value: String) -> Self {
                if value.trim().is_empty() {
                    Self(String::from("unspecified rule boundary failure"))
                } else {
                    Self(value)
                }
            }
        }

        impl TryFrom<String> for $name {
            type Error = DecodeError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.trim().is_empty() || ($field != "ruleCatalogJson" && $field != "ruleManifestJson" && $field != "waiverDocumentJson" && value.chars().any(char::is_control)) {
                    return Err(DecodeError::new($field, "must be non-empty printable text"));
                }
                Ok(Self(value))
            }
        }

        impl std::str::FromStr for $name {
            type Err = DecodeError;
            // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
            fn from_str(value: &str) -> Result<Self, Self::Err> { Self::try_from(value.to_owned()) }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self { value.0 }
        }
    };
}

rule_text!(
    #[doc = "Human-readable rule title."]
    RuleTitle,
    "ruleTitle"
);
rule_text!(
    #[doc = "Validator implementation path."]
    ValidatorPath,
    "validatorPath"
);
rule_text!(
    #[doc = "Documentation anchor for a rule."]
    RuleDocAnchor,
    "ruleDocAnchor"
);
rule_text!(
    #[doc = "A rule-family grouping tag."]
    RuleTag,
    "ruleTag"
);
rule_text!(
    #[doc = "Accountable owner for a narrow waiver."]
    WaiverOwner,
    "waiverOwner"
);
rule_text!(
    #[doc = "Concrete reason for a narrow waiver."]
    WaiverReason,
    "waiverReason"
);
rule_text!(
    #[doc = "Human-readable catalog source label."]
    RuleCatalogSource,
    "ruleCatalogSource"
);
rule_text!(
    #[doc = "JSON text received at the rule-catalog boundary."]
    RuleCatalogJson,
    "ruleCatalogJson"
);
rule_text!(
    #[doc = "JSON text received at the rule-manifest boundary."]
    RuleManifestJson,
    "ruleManifestJson"
);
rule_text!(
    #[doc = "Human-readable source label for a waiver document."]
    WaiverDocumentSource,
    "waiverDocumentSource"
);
rule_text!(
    #[doc = "JSON text received at the waiver-document boundary."]
    WaiverDocumentJson,
    "waiverDocumentJson"
);
rule_text!(
    #[doc = "Failure explanation emitted by the rule boundary."]
    RuleFailureReason,
    "ruleFailureReason"
);
rule_text!(
    #[doc = "A key in a rule-specific parameter object."]
    RuleParameterKey,
    "ruleParameterKey"
);
rule_text!(
    #[doc = "A text value retained from a rule-specific parameter object."]
    RuleParameterText,
    "ruleParameterText"
);

/// Positive semantic version assigned to one rule record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for RuleVersion."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct RuleVersion(NonZeroU32);

impl RuleVersion {
    /// Brand an already validated nonzero record version.
    #[must_use]
    pub const fn try_new(value: NonZeroU32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> NonZeroU32 {
        self.0
    }
}

/// Positive schema version for the rule-version manifest document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for RuleManifestSchemaVersion."]
pub struct RuleManifestSchemaVersion(NonZeroU32);

impl RuleManifestSchemaVersion {
    /// Brand an already validated nonzero manifest schema version.
    #[must_use]
    pub const fn try_new(value: NonZeroU32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> NonZeroU32 {
        self.0
    }
}

/// One canonical version-manifest entry, pairing a rule version with the
/// digest of the parity artifacts it pins.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for RuleManifestEntry."]
pub struct RuleManifestEntry {
    version: RuleVersion,
    hash: Sha256,
}

impl RuleManifestEntry {
    /// Construct a validated manifest entry from already-branded values.
    #[must_use]
    #[doc = "The new operation for this canonical domain value."]
    pub fn new(version: RuleVersion, hash: Sha256) -> Self {
        Self { version, hash }
    }

    /// Read the pinned rule version.
    #[must_use]
    #[doc = "The version operation for this canonical domain value."]
    pub fn version(&self) -> RuleVersion {
        self.version
    }

    /// Read the pinned parity-artifact digest.
    #[must_use]
    #[doc = "The hash operation for this canonical domain value."]
    pub fn hash(&self) -> &Sha256 {
        &self.hash
    }
}

/// Typed rule-version manifest after JSON ingress has completed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for RuleManifest."]
pub struct RuleManifest {
    schema_version: RuleManifestSchemaVersion,
    entries: BTreeMap<RuleId, RuleManifestEntry>,
}

impl RuleManifest {
    /// Construct from a validated schema version and canonical entries.
    #[must_use]
    #[doc = "The new operation for this canonical domain value."]
    pub fn new(
        schema_version: RuleManifestSchemaVersion,
        entries: BTreeMap<RuleId, RuleManifestEntry>,
    ) -> Self {
        Self {
            schema_version,
            entries,
        }
    }

    /// Read the wire schema version.
    #[must_use]
    #[doc = "The schema_version operation for this canonical domain value."]
    pub fn schema_version(&self) -> RuleManifestSchemaVersion {
        self.schema_version
    }

    /// Look up one pinned rule entry.
    #[must_use]
    #[doc = "The entry operation for this canonical domain value."]
    pub fn entry(&self, rule_id: &RuleId) -> Option<&RuleManifestEntry> {
        self.entries.get(rule_id)
    }

    /// Number of pinned rule entries.
    #[must_use]
    pub fn count(&self) -> RuleRecordCount {
        RuleRecordCount::from_records(self.entries.keys())
    }

    /// Iterate every pinned rule id in deterministic order.
    pub fn rule_ids(&self) -> impl Iterator<Item = &RuleId> {
        self.entries.keys()
    }
}

/// Validated inclusive UTC calendar date used to expire a narrow waiver.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for WaiverExpiryDate."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct WaiverExpiryDate(String);

impl WaiverExpiryDate {
    /// Validate a real `YYYY-MM-DD` date, rejecting invalid calendar input.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let bytes = value.as_bytes();
        if bytes.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
            return Err(DecodeError::new("waiverExpiryDate", "must use YYYY-MM-DD"));
        }
        let year = value
            .get(..4)
            .ok_or_else(|| DecodeError::new("waiverExpiryDate", "year boundary is invalid"))?
            .parse::<u16>()
            .map_err(|_year_error| DecodeError::new("waiverExpiryDate", "year must be numeric"))?;
        let month = value
            .get(5..7)
            .ok_or_else(|| DecodeError::new("waiverExpiryDate", "month boundary is invalid"))?
            .parse::<u8>()
            .map_err(|_month_error| {
                DecodeError::new("waiverExpiryDate", "month must be numeric")
            })?;
        let day = value
            .get(8..10)
            .ok_or_else(|| DecodeError::new("waiverExpiryDate", "day boundary is invalid"))?
            .parse::<u8>()
            .map_err(|_day_error| DecodeError::new("waiverExpiryDate", "day must be numeric"))?;
        let valid_month = (1..=12).contains(&month);
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
            2 => 28,
            _ => 0,
        };
        if !valid_month || !(1..=days).contains(&day) {
            return Err(DecodeError::new(
                "waiverExpiryDate",
                "must name a real calendar day",
            ));
        }
        Ok(Self(value))
    }

    /// Expose canonical spelling for serialization at the boundary.
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WaiverExpiryDate {
    type Error = DecodeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl std::str::FromStr for WaiverExpiryDate {
    type Err = DecodeError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
        Self::try_new(value.to_owned())
    }
}

impl std::fmt::Display for WaiverExpiryDate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Pure, recursively typed rule-specific parameters retained after JSON ingress.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for RuleParameter."]
pub enum RuleParameter {
    Null,
    Boolean(bool),
    Integer(i64),
    Unsigned(u64),
    Text(RuleParameterText),
    List(Vec<Self>),
    Object(BTreeMap<RuleParameterKey, Self>),
}

/// Opaque-but-typed parameter object for a rule family.
#[derive(Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for RuleParameters."]
pub struct RuleParameters(BTreeMap<RuleParameterKey, RuleParameter>);

impl std::fmt::Debug for RuleParameters {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RuleParameters([REDACTED])")
    }
}

impl RuleParameters {
    /// Construct from a fully validated parameter map at the boundary.
    #[must_use]
    #[doc = "The new operation for this canonical domain value."]
    pub fn new(values: BTreeMap<RuleParameterKey, RuleParameter>) -> Self {
        Self(values)
    }

    /// Obtain a parameter by its canonical key.
    #[must_use]
    #[doc = "The get operation for this canonical domain value."]
    pub fn get(&self, key: &str) -> Option<&RuleParameter> {
        self.0
            .iter()
            .find_map(|(candidate, value)| (candidate.as_str() == key).then_some(value))
    }
}
