//! Canonical pure values used by the structured rule registry.
//!
//! JSON catalog spelling belongs to `enforcer-rules::boundary`; these values
//! carry only validated domain meaning after the catalog boundary.

use crate::boundary::decode_error::DecodeError;
use crate::hashes::Sha256;
use crate::ids::RuleId;

/// Canonical outcome of comparing two versions of one rule record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionDriftOutcome {
    Unchanged,
    ContentChangedVersionBumped,
    ContentChangedVersionNotBumped,
    VersionBumpedContentUnchanged,
    RuleIdMismatch,
}

impl VersionDriftOutcome {
    #[must_use]
    pub fn is_drift(&self) -> bool {
        matches!(
            self,
            Self::ContentChangedVersionNotBumped
                | Self::VersionBumpedContentUnchanged
                | Self::RuleIdMismatch
        )
    }
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
            pub fn as_str(&self) -> &str { &self.0 }
            #[must_use]
            pub fn is_empty(&self) -> bool { self.0.is_empty() }
            #[must_use]
            pub fn starts_with(&self, needle: &str) -> bool { self.0.starts_with(needle) }
            #[must_use]
            pub fn contains(&self, needle: &str) -> bool { self.0.contains(needle) }
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
        impl PartialEq<&str> for $name { fn eq(&self, value: &&str) -> bool { self.0 == *value } }
        impl PartialEq<str> for $name { fn eq(&self, value: &str) -> bool { self.0 == value } }
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
pub struct RuleVersion(u32);

impl RuleVersion {
    /// Build a nonzero record version.
    pub fn new(value: u32) -> Result<Self, DecodeError> {
        if value == 0 {
            return Err(DecodeError::new("ruleVersion", "must be greater than zero"));
        }
        Ok(Self(value))
    }

    /// View the version's numeric value.
    #[must_use]
    pub fn value(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for RuleVersion {
    type Error = DecodeError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Positive schema version for the rule-version manifest document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleManifestSchemaVersion(NonZeroU32);

impl RuleManifestSchemaVersion {
    /// Build a nonzero manifest schema version.
    pub fn new(value: u32) -> Result<Self, DecodeError> {
        NonZeroU32::new(value).map(Self).ok_or_else(|| {
            DecodeError::new("ruleManifestSchemaVersion", "must be greater than zero")
        })
    }

    /// View the numeric schema version at the serialization boundary.
    #[must_use]
    pub fn value(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for RuleManifestSchemaVersion {
    type Error = DecodeError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// One canonical version-manifest entry, pairing a rule version with the
/// digest of the parity artifacts it pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleManifestEntry {
    version: RuleVersion,
    hash: Sha256,
}

impl RuleManifestEntry {
    /// Construct a validated manifest entry from already-branded values.
    #[must_use]
    pub fn new(version: RuleVersion, hash: Sha256) -> Self {
        Self { version, hash }
    }

    /// Read the pinned rule version.
    #[must_use]
    pub fn version(&self) -> RuleVersion {
        self.version
    }

    /// Read the pinned parity-artifact digest.
    #[must_use]
    pub fn hash(&self) -> &Sha256 {
        &self.hash
    }
}

/// Typed rule-version manifest after JSON ingress has completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleManifest {
    schema_version: RuleManifestSchemaVersion,
    entries: BTreeMap<RuleId, RuleManifestEntry>,
}

impl RuleManifest {
    /// Construct from a validated schema version and canonical entries.
    #[must_use]
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
    pub fn schema_version(&self) -> RuleManifestSchemaVersion {
        self.schema_version
    }

    /// Look up one pinned rule entry.
    #[must_use]
    pub fn entry(&self, rule_id: &RuleId) -> Option<&RuleManifestEntry> {
        self.entries.get(rule_id)
    }

    /// Number of pinned rule entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Iterate every pinned rule id in deterministic order.
    pub fn rule_ids(&self) -> impl Iterator<Item = &RuleId> {
        self.entries.keys()
    }
}

/// Validated inclusive UTC calendar date used to expire a narrow waiver.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WaiverExpiryDate(String);

impl WaiverExpiryDate {
    /// Parse a real `YYYY-MM-DD` date at the boundary.
    pub fn parse(value: String) -> Result<Self, DecodeError> {
        let bytes = value.as_bytes();
        if bytes.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
            return Err(DecodeError::new("waiverExpiryDate", "must use YYYY-MM-DD"));
        }
        let year = value[..4]
            .parse::<u16>()
            .map_err(|_| DecodeError::new("waiverExpiryDate", "year must be numeric"))?;
        let month = value[5..7]
            .parse::<u8>()
            .map_err(|_| DecodeError::new("waiverExpiryDate", "month must be numeric"))?;
        let day = value[8..10]
            .parse::<u8>()
            .map_err(|_| DecodeError::new("waiverExpiryDate", "day must be numeric"))?;
        let valid_month = (1..=12).contains(&month);
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if year.is_multiple_of(4)
                && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
            {
                29
            }
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
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WaiverExpiryDate {
    type Error = DecodeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl std::str::FromStr for WaiverExpiryDate {
    type Err = DecodeError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value.to_owned())
    }
}

impl std::fmt::Display for WaiverExpiryDate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Pure, recursively typed rule-specific parameters retained after JSON ingress.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuleParameters(BTreeMap<RuleParameterKey, RuleParameter>);

impl RuleParameters {
    /// Construct from a fully validated parameter map at the boundary.
    #[must_use]
    pub fn new(values: BTreeMap<RuleParameterKey, RuleParameter>) -> Self {
        Self(values)
    }

    /// Obtain a parameter by its canonical key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&RuleParameter> {
        self.0
            .iter()
            .find_map(|(candidate, value)| (candidate.as_str() == key).then_some(value))
    }
}
