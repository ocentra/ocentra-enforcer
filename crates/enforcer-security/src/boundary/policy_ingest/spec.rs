//! Typed record shapes for a parsed policy spec and its ingest result
//! (h08, POLICY-SPEC-INGESTION).
//!
//! [`super::parse::parse_spec`] turns raw `.mdc` text into a [`PolicySpec`];
//! [`super::map::map_to_profile`] maps a [`PolicySpec`] against the backed
//! [`super::backing::BackedRuleCatalog`] to produce a [`MechanizedProfile`]
//! plus [`crate::policy_ingest::Finding`]-shaped flags for anything
//! asserted-but-unbacked. Both stages read/write only canonical
//! `enforcer-domain` values after raw input is decoded, without plain
//! `String`/`Vec` scaffolding — never an ad-hoc stringly-typed map.
//! BOUNDARY-INVARIANT: serde DTOs are limited to the persisted profile wire
//! shape while parsed rule identifiers and tiers remain canonical domain values.
//! NEGATIVE-TEST: tests/policy_ingest.rs rejects malformed and conflicting
//! policy entries before any profile is constructed.

use std::collections::BTreeSet;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::ConfigProfileName;
use enforcer_domain::ids::RuleId;
use enforcer_domain::security_types::{SecurityInvariantId, SecurityTestCategory};
use enforcer_domain::severity::{Severity, Tier};
use enforcer_domain::telemetry_types::SourceLine;

/// One rule a spec asserts: its id, the mechanical tier the spec claims for
/// it (T1 block / T2 score / T3 label), and the source line it came from
/// (1-based, for `Finding` pointers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertedRule {
    /// The rule id token the spec asserted (e.g. `MCM-SIGNING.1`).
    pub rule_id: RuleId,
    /// The tier the spec claims this rule should be enforced at.
    pub tier: Tier,
    /// 1-based line number in the ingested spec text.
    pub line: SourceLine,
}

/// The typed result of parsing a project's `.mdc` policy-spec text: the
/// required test categories (§3.1-3.20 in the reference spec), the
/// invariant ids (§2.3), and the rules it asserts (mapped in the next
/// stage against the backed-rule catalog).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PolicySpec {
    /// Required test category names, in spec order, de-duplicated.
    pub required_test_categories: Vec<SecurityTestCategory>,
    /// Invariant ids (kebab-case, e.g. `failure-not-reward`), in spec
    /// order, de-duplicated.
    pub invariants: Vec<SecurityInvariantId>,
    /// Every rule the spec asserts, in spec order.
    pub asserted_rules: Vec<AssertedRule>,
}

/// Fully validated runtime security profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MechanizedProfile {
    /// Validated neutral profile name.
    pub profile_name: ConfigProfileName,
    /// Required test categories decoded from policy input.
    pub required_test_categories: Vec<SecurityTestCategory>,
    /// Required invariant identifiers decoded from policy input.
    pub invariants: Vec<SecurityInvariantId>,
    /// Mechanized rule rows.
    pub rules: Vec<ProfileRuleRow>,
}

/// One row of a [`MechanizedProfile`]: a rule id paired with the severity
/// tier the profile enforces it at, and whether it is actually ENABLED
/// (backed by a real `Validator`) or only FLAGGED for mechanization
/// (asserted by the spec, no backing validator exists yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRuleRow {
    /// The rule id.
    pub rule_id: RuleId,
    /// The tier this profile enforces the rule at.
    pub tier: Tier,
    /// `true` when a real mechanized `Validator` backs this rule id (the
    /// row is actually enforced); `false` when the spec asserts the rule
    /// but no validator backs it (visibly un-enforced, never silently
    /// treated as if it were).
    pub backed: bool,
}

/// Persisted transport form of a mapped [`MechanizedProfile`]. Carries NO
/// product/company/game branding — every string is a generic doctrine term
/// (money-critical, invariant, threat category), never a project name.
// ROUNDTRIP-TEST: tests/policy_ingest.rs profile_shape verifies full-profile
// JSON decode/encode/decode stability, including every rule row.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[doc = "Persisted mechanized security profile transport record."]
/// Persisted mechanized security profile transport record.
pub struct MechanizedProfileDto {
    /// Neutral profile name (no product/company/game branding).
    #[serde(rename = "profileName")]
    pub profile_name: String,
    /// Required test category names this profile expects money-critical
    /// units to cover.
    #[serde(rename = "requiredTestCategories")]
    pub required_test_categories: Vec<String>,
    /// Required economic/logic invariant ids this profile expects.
    pub invariants: Vec<String>,
    /// Every rule row (id + tier + backed) this profile carries. Includes
    /// both ENABLED (backed) rows and rows that are only asserted (see
    /// [`MechanizedProfileDto::unbacked_rule_ids`] for the flagged subset).
    pub rules: Vec<ProfileRuleRowDto>,
}

/// Wire (`serde`) shape for [`ProfileRuleRow`] — `RuleId`/`Tier` already
/// carry their own `serde` impls; this wrapper adds the plain `backed`
/// bool the JSON profile format commits to disk.
// ROUNDTRIP-TEST: covered as a nested external shape by profile_shape.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[doc = "Persisted transport row for one mechanized security rule."]
/// Persisted transport row for one mechanized security rule.
pub struct ProfileRuleRowDto {
    /// The rule id, wire form.
    #[serde(rename = "ruleId")]
    pub rule_id: RuleId,
    /// The severity tier, wire form (`"T1"`/`"T2"`/`"T3"`).
    pub tier: Tier,
    /// Same-name field per [`ProfileRuleRow::backed`].
    pub backed: bool,
}

impl From<ProfileRuleRow> for ProfileRuleRowDto {
    fn from(row: ProfileRuleRow) -> Self {
        Self {
            rule_id: row.rule_id,
            tier: row.tier,
            backed: row.backed,
        }
    }
}

impl From<ProfileRuleRowDto> for ProfileRuleRow {
    fn from(row: ProfileRuleRowDto) -> Self {
        Self {
            rule_id: row.rule_id,
            tier: row.tier,
            backed: row.backed,
        }
    }
}

impl TryFrom<MechanizedProfileDto> for MechanizedProfile {
    type Error = DecodeError;

    fn try_from(profile: MechanizedProfileDto) -> Result<Self, Self::Error> {
        Ok(Self {
            profile_name: ConfigProfileName::try_new(profile.profile_name)?,
            required_test_categories: profile
                .required_test_categories
                .into_iter()
                .map(SecurityTestCategory::try_from)
                .collect::<Result<_, _>>()?,
            invariants: profile
                .invariants
                .into_iter()
                .map(SecurityInvariantId::try_from)
                .collect::<Result<_, _>>()?,
            rules: profile.rules.into_iter().map(Into::into).collect(),
        })
    }
}

impl From<&MechanizedProfile> for MechanizedProfileDto {
    fn from(profile: &MechanizedProfile) -> Self {
        Self {
            profile_name: profile.profile_name.as_str().to_owned(),
            required_test_categories: profile
                .required_test_categories
                .iter()
                .cloned()
                .map(String::from)
                .collect(),
            invariants: profile
                .invariants
                .iter()
                .cloned()
                .map(String::from)
                .collect(),
            rules: profile.rules.iter().cloned().map(Into::into).collect(),
        }
    }
}

impl MechanizedProfile {
    /// Rule ids in this profile that are asserted but NOT backed by a real
    /// validator — the honesty-seam subset that must be flagged for
    /// mechanization (fed to d01/d08), never silently treated as enabled.
    pub fn unbacked_rule_ids(&self) -> BTreeSet<&str> {
        self.rules
            .iter()
            .filter(|row| !row.backed)
            .map(|row| row.rule_id.as_str())
            .collect()
    }

    /// Severity for wire-level convenience: `T1` maps to a blocking
    /// [`Severity::Error`], `T2` to [`Severity::Warning`] (scored), `T3` to
    /// [`Severity::Info`] (advisory label) — matching the doctrine's
    /// T1-block/T2-score/T3-label convention.
    pub const fn tier_severity(tier: Tier) -> Severity {
        match tier {
            Tier::T1 => Severity::Error,
            Tier::T2 => Severity::Warning,
            Tier::T3 => Severity::Info,
        }
    }
}
