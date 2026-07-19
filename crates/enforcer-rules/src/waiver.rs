//! Typed, fail-closed waivers for individual findings.
//!
//! This is intentionally not a second policy-toggle format. A waiver names
//! one known [`RuleId`], one exact repository-relative path, an accountable
//! owner, and a non-empty reason. Expired waivers never match a finding.

use std::collections::BTreeSet;
use std::path::Path;

use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::{
    RuleFailureReason, WaiverDocumentJson, WaiverDocumentSource, WaiverExpiryDate, WaiverOwner,
    WaiverReason,
};

use crate::registry::RuleRegistry;

/// Determines whether an expired registry entry rejects the full load or is
/// retained for audit while remaining ineligible to match a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryPolicy {
    /// Reject a registry containing an expired entry.
    RejectExpired,
    /// Retain expired entries for audit. They still never match a finding.
    RetainExpiredForAudit,
}

/// One auditable exception for one rule and one exact repository-relative
/// path. The shape cannot represent a numeric policy limit bump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiver {
    /// Exact repository-relative file path the waiver applies to.
    pub path: RelPath,
    /// Branded identifier for the specific waived rule.
    pub rule_id: RuleId,
    /// Accountable human or team.
    pub owner: WaiverOwner,
    /// Concrete reason this one finding is temporarily accepted.
    pub reason: WaiverReason,
    /// Optional inclusive UTC expiry date.
    pub expires: Option<WaiverExpiryDate>,
}

/// The on-disk waiver document. JSON is used because the crate already owns
/// a serde JSON loader convention; no RON dependency or workspace lockfile
/// mutation is needed for this standalone registry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WaiverRegistry {
    /// Every tracked waiver entry.
    waivers: Vec<Waiver>,
}

impl WaiverRegistry {
    /// Construct from entries decoded at this crate's waiver boundary.
    pub(crate) fn new(waivers: Vec<Waiver>) -> Self {
        Self { waivers }
    }

    /// Iterate the tracked waiver entries without exposing collection storage.
    pub fn iter(&self) -> impl Iterator<Item = &Waiver> {
        self.waivers.iter()
    }

    /// Replace the waiver for the same exact path and rule, or append it.
    pub fn upsert(&mut self, waiver: Waiver) {
        self.waivers
            .retain(|existing| existing.path != waiver.path || existing.rule_id != waiver.rule_id);
        self.waivers.push(waiver);
    }

    /// Parse, validate, and return a registry. A partially valid document
    /// never escapes this boundary.
    pub fn parse(
        raw: &WaiverDocumentJson,
        source: &WaiverDocumentSource,
        rules: &RuleRegistry,
        today: &WaiverExpiryDate,
        expiry_policy: ExpiryPolicy,
    ) -> WaiverResult<Self> {
        let registry = crate::boundary::waiver::decode(raw, source)?;
        registry.validate(rules, today, expiry_policy)?;
        Ok(registry)
    }

    /// Read, parse, and validate a registry from disk.
    pub fn load_file(
        path: &Path,
        rules: &RuleRegistry,
        today: &WaiverExpiryDate,
        expiry_policy: ExpiryPolicy,
    ) -> WaiverResult<Self> {
        // ALLOC-JUSTIFICATION: the boundary error must own a stable rendered path after this call.
        let display =
            WaiverDocumentSource::try_from(path.display().to_string()).map_err(|error| {
                WaiverLoadError::InvalidPath {
                    detail: crate::boundary_reason(error),
                }
            })?;
        let raw = std::fs::read_to_string(path).map_err(|error| WaiverLoadError::Io {
            // CLONE-JUSTIFICATION: the error owns its source while the success path retains it for parsing.
            catalog_source: display.clone(),
            reason: crate::boundary_reason(error),
        })?;
        let raw =
            WaiverDocumentJson::try_from(raw).map_err(|error| WaiverLoadError::InvalidPath {
                detail: crate::boundary_reason(error),
            })?;
        Self::parse(&raw, &display, rules, today, expiry_policy)
    }

    /// Validate all semantic constraints that serde alone cannot express.
    pub fn validate(
        &self,
        rules: &RuleRegistry,
        today: &WaiverExpiryDate,
        expiry_policy: ExpiryPolicy,
    ) -> WaiverResult<()> {
        let mut entries = BTreeSet::new();
        for waiver in &self.waivers {
            let path = canonical_relative_path(&waiver.path)?;
            if rules.get(&waiver.rule_id).is_none() {
                return Err(WaiverLoadError::UnknownRuleId {
                    // CLONE-JUSTIFICATION: validation borrows the registry while the returned error owns context.
                    path: waiver.path.clone(),
                    // CLONE-JUSTIFICATION: validation borrows the registry while the returned error owns context.
                    rule_id: waiver.rule_id.clone(),
                });
            }
            if matches!(expiry_policy, ExpiryPolicy::RejectExpired)
                && waiver
                    .expires
                    .as_ref()
                    .is_some_and(|expires| expires < today)
            {
                return Err(WaiverLoadError::Expired {
                    // CLONE-JUSTIFICATION: validation borrows the registry while the returned error owns context.
                    path: waiver.path.clone(),
                    // CLONE-JUSTIFICATION: validation borrows the registry while the returned error owns context.
                    rule_id: waiver.rule_id.clone(),
                    // CLONE-JUSTIFICATION: the returned error owns the validated expiry from the borrowed waiver.
                    expires: waiver.expires.clone().ok_or_else(|| {
                        WaiverLoadError::InvalidExpiry {
                            value: crate::boundary_reason(
                                "expired waiver must have an expiry date",
                            ),
                        }
                    })?,
                });
            }
            // CLONE-JUSTIFICATION: the uniqueness set owns its key while the borrowed waiver remains available.
            if !entries.insert((path, waiver.rule_id.clone())) {
                return Err(WaiverLoadError::DuplicateScope {
                    // CLONE-JUSTIFICATION: validation borrows the registry while the returned error owns context.
                    path: waiver.path.clone(),
                    // CLONE-JUSTIFICATION: validation borrows the registry while the returned error owns context.
                    rule_id: waiver.rule_id.clone(),
                });
            }
        }
        Ok(())
    }

    /// Return the single active waiver applicable to this exact path and
    /// rule. Invalid candidate paths and expired entries fail closed.
    pub fn matching<'a>(
        &'a self,
        path: &RelPath,
        rule_id: &RuleId,
        today: &WaiverExpiryDate,
    ) -> Option<&'a Waiver> {
        self.waivers.iter().find(|waiver| {
            waiver.rule_id == *rule_id
                && waiver.path == *path
                && waiver
                    .expires
                    .as_ref()
                    .is_none_or(|expires| expires >= today)
        })
    }
}

/// Fail-closed result for waiver registry loading and validation.
pub type WaiverResult<T> = Result<T, WaiverLoadError>;

/// A registry boundary failure.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum WaiverLoadError {
    /// The document could not be read.
    #[error("failed to read waiver registry `{catalog_source}`: {reason}")]
    Io {
        catalog_source: WaiverDocumentSource,
        reason: RuleFailureReason,
    },
    /// The document did not decode into the strict registry shape.
    #[error("waiver registry parse failed at `{catalog_source}`: {reason}")]
    Parse {
        catalog_source: WaiverDocumentSource,
        reason: RuleFailureReason,
    },
    /// An expiry date did not use a real `YYYY-MM-DD` calendar date.
    #[error("invalid waiver expiry `{value}`; expected a real YYYY-MM-DD date")]
    InvalidExpiry { value: RuleFailureReason },
    /// A waiver path was broad, absolute, or escaped the project root.
    #[error("waiver path rejected: {detail}")]
    InvalidPath { detail: RuleFailureReason },
    /// A waiver omitted accountable ownership.
    #[error("waiver for `{path}` / `{rule_id}` has an empty owner")]
    EmptyOwner { path: RelPath, rule_id: RuleId },
    /// A waiver omitted an auditable reason.
    #[error("waiver for `{path}` / `{rule_id}` has an empty reason")]
    EmptyReason { path: RelPath, rule_id: RuleId },
    /// A syntactically valid identifier was not present in the rule registry.
    #[error("waiver for `{path}` references unknown rule `{rule_id}`")]
    UnknownRuleId { path: RelPath, rule_id: RuleId },
    /// A strict load rejected a waiver after its expiry date.
    #[error("waiver for `{path}` / `{rule_id}` expired on {expires}")]
    Expired {
        path: RelPath,
        rule_id: RuleId,
        expires: WaiverExpiryDate,
    },
    /// Two entries tried to waive the same rule for the same path.
    #[error("duplicate waiver scope for `{path}` / `{rule_id}`")]
    DuplicateScope { path: RelPath, rule_id: RuleId },
}

fn canonical_relative_path(raw: &RelPath) -> WaiverResult<RelPath> {
    let normalized = raw.as_str().trim().replace('\\', "/");
    let normalized = normalized.trim_start_matches("./");
    let invalid = normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.contains(':')
        || normalized.contains('*')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..");
    if invalid {
        return Err(WaiverLoadError::InvalidPath {
            detail: crate::boundary_reason(raw.as_str()),
        });
    }
    // ALLOC-JUSTIFICATION: the normalized canonical path owns text derived from the input.
    RelPath::try_from(normalized.to_owned()).map_err(|error| WaiverLoadError::InvalidPath {
        detail: crate::boundary_reason(error),
    })
}

#[cfg(test)]
mod property_tests {
    use super::{ExpiryPolicy, WaiverRegistry};
    use crate::registry::RuleRegistry;
    use enforcer_domain::rules_types::{
        WaiverDocumentJson, WaiverDocumentSource, WaiverExpiryDate,
    };
    use proptest::{prop_assert, proptest};

    proptest! {
        #[test]
        fn parse_rejects_malformed_generated_unknown_top_level_fields(field in "[a-z]{1,16}") {
            let raw = match WaiverDocumentJson::try_from(
                serde_json::json!({field: []}).to_string()
            ) {
                Ok(raw) => raw,
                Err(_) => return Err(proptest::test_runner::TestCaseError::fail("waiver JSON wrapper rejected generated JSON")),
            };
            let source = match WaiverDocumentSource::try_from("generated waivers".to_owned()) {
                Ok(source) => source,
                Err(_) => return Err(proptest::test_runner::TestCaseError::fail("waiver source rejected static text")),
            };
            let today = match "2026-07-16".parse::<WaiverExpiryDate>() {
                Ok(today) => today,
                Err(_) => return Err(proptest::test_runner::TestCaseError::fail("static waiver date was invalid")),
            };
            let rules = match RuleRegistry::from_records(Vec::new()) {
                Ok(rules) => rules,
                Err(_) => return Err(proptest::test_runner::TestCaseError::fail("empty registry construction failed")),
            };
            let outcome = WaiverRegistry::parse(
                &raw,
                &source,
                &rules,
                &today,
                ExpiryPolicy::RejectExpired,
            );
            let rejected_as_parse_error =
                matches!(outcome, Err(crate::waiver::WaiverLoadError::Parse { .. }));
            prop_assert!(rejected_as_parse_error);
        }
    }
}
