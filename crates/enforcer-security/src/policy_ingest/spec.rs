//! Typed record shapes for a parsed policy spec and its ingest result
//! (h08, POLICY-SPEC-INGESTION).
//!
//! [`super::parse::parse_spec`] turns raw `.mdc` text into a [`PolicySpec`];
//! [`super::map::map_to_profile`] maps a [`PolicySpec`] against the backed
//! [`super::backing::BackedRuleCatalog`] to produce a [`MechanizedProfile`]
//! plus [`crate::policy_ingest::Finding`]-shaped flags for anything
//! asserted-but-unbacked. Both stages read/write only
//! `enforcer-domain` newtypes (`RuleId`, `Severity`, `Tier`) plus plain
//! `String`/`Vec` scaffolding — never an ad-hoc stringly-typed map.

use std::collections::BTreeSet;

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::{Severity, Tier};

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
    pub line: u32,
}

/// The typed result of parsing a project's `.mdc` policy-spec text: the
/// required test categories (§3.1-3.20 in the reference spec), the
/// invariant ids (§2.3), and the rules it asserts (mapped in the next
/// stage against the backed-rule catalog).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PolicySpec {
    /// Required test category names, in spec order, de-duplicated.
    pub required_test_categories: Vec<String>,
    /// Invariant ids (kebab-case, e.g. `failure-not-reward`), in spec
    /// order, de-duplicated.
    pub invariants: Vec<String>,
    /// Every rule the spec asserts, in spec order.
    pub asserted_rules: Vec<AssertedRule>,
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

/// The mechanized profile produced by mapping a [`PolicySpec`] against the
/// backed-rule catalog: the neutral, loadable, `serde`-typed record that
/// `enforcer-config`/`enforcer-rules` load and the UI renders. Carries NO
/// product/company/game branding — every string is a generic doctrine term
/// (money-critical, invariant, threat category), never a project name.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct MechanizedProfile {
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
    /// [`MechanizedProfile::unbacked_rule_ids`] for the flagged subset).
    pub rules: Vec<ProfileRuleRowWire>,
}

/// Wire (`serde`) shape for [`ProfileRuleRow`] — `RuleId`/`Tier` already
/// carry their own `serde` impls; this wrapper adds the plain `backed`
/// bool the JSON profile format commits to disk.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProfileRuleRowWire {
    /// The rule id, wire form.
    #[serde(rename = "ruleId")]
    pub rule_id: RuleId,
    /// The severity tier, wire form (`"T1"`/`"T2"`/`"T3"`).
    pub tier: Tier,
    /// Same-name field per [`ProfileRuleRow::backed`].
    pub backed: bool,
}

impl From<ProfileRuleRow> for ProfileRuleRowWire {
    fn from(row: ProfileRuleRow) -> Self {
        Self {
            rule_id: row.rule_id,
            tier: row.tier,
            backed: row.backed,
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
    pub fn tier_severity(tier: Tier) -> Severity {
        match tier {
            Tier::T1 => Severity::Error,
            Tier::T2 => Severity::Warning,
            Tier::T3 => Severity::Info,
        }
    }
}
