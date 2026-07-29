//! Declarative policy externalization (f03): owner/exempt globs,
//! allow-regex lists, `cfg(test)`/test-path skipping, and per-rule toggles
//! (enable/disable, severity override, waiver) — all data read from
//! `.enforce/config`, NEVER an inline `#[allow]`/comment disable in the
//! target source (that inline path stays banned; enforced elsewhere by the
//! no-bypass meta-check in `enforcer-security`).
//!
//! This is the OcentraParent config-externalization borrow: every exception
//! is a committed, reviewable, attributable line of policy data, not a
//! silent escape hatch buried next to the code it exempts.

use enforcer_domain::{
    config_types::{CfgTestSkipping, Glob, PolicyOwner, PolicyReason, RegexPattern, RuleEnabled},
    ids::RuleId,
};
use std::collections::BTreeMap;

/// A single per-rule override: enable/disable, severity override, and/or a
/// waiver. Disabling a rule outright still requires [`Waiver`] fields
/// (owner + reason) per the honesty doctrine — there is no bare `false`
/// escape hatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleToggle {
    /// Whether the rule is enabled. Defaults to `true`: absence of a toggle
    /// entry never disables a rule.
    pub enabled: RuleEnabled,
    /// Optional severity override for this rule.
    pub severity: Option<enforcer_domain::severity::Severity>,
    /// Required when `enabled = false`: the attributable waiver record.
    /// Absence while `enabled = false` is a boundary error (see
    /// [`crate::error::ConfigLoadError`]), not a silent disable.
    pub waiver: Option<Waiver>,
}

/// An attributable waiver: who granted the exception and why. Required to
/// disable a rule; there is no anonymous or reasonless disable path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiver {
    /// The rule this waiver applies to (redundant with the map key, kept for
    /// self-description when a waiver record is extracted/logged alone).
    pub rule_id: RuleId,
    /// Who owns/granted this waiver (person or team handle, free text).
    pub owner: PolicyOwner,
    /// Why the rule is waived for this project. Must be non-empty (checked
    /// at boundary).
    pub reason: PolicyReason,
}

/// The full declarative policy externalization surface. Every field here is
/// data read from `.enforce/config`; there is no code path that lets a
/// target source file locally suppress a rule instead.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Policy {
    /// Globs marking files whose owning team/individual is committed policy
    /// (not tribal knowledge).
    pub owner_globs: Vec<Glob>,
    /// Globs exempt from enforcement entirely (e.g. generated code,
    /// vendored sources). Distinct from a rule-level waiver: this exempts
    /// the *path*, not a *rule*.
    pub exempt_globs: Vec<Glob>,
    /// Regex source patterns (escaped) that, when a line/path matches, are
    /// allowed despite an otherwise-matching rule.
    pub allow_regex: Vec<RegexPattern>,
    /// Whether `#[cfg(test)]` modules and conventional test-path globs are
    /// skipped by scope-sensitive rules.
    pub skip_cfg_test: CfgTestSkipping,
    /// Additional test-path globs skipped alongside `#[cfg(test)]` (e.g.
    /// `tests/**`, `**/*_test.rs`) when [`Policy::skip_cfg_test`] is set.
    pub test_path_globs: Vec<Glob>,
    /// Per-rule toggles, keyed by [`RuleId`]. Absence of a key means the
    /// rule runs at its default severity — never silently disabled by
    /// omission.
    pub rule_toggles: BTreeMap<RuleId, RuleToggle>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PolicyValidationError {
    #[error("a disabled rule has no attributable waiver")]
    DisabledRuleWithoutWaiver,
    #[error("a waiver rule id does not match its map key")]
    WaiverRuleIdMismatch,
}

impl Policy {
    /// Validate cross-field invariants that plain `serde` shape checking
    /// cannot express: a disabled rule must carry a non-empty waiver
    /// (owner + reason), matching the honesty doctrine — no silent
    /// suppression.
    ///
    /// # Errors
    /// Returns a description of the first invariant violation found.
    pub fn validate(&self) -> Result<(), PolicyValidationError> {
        for (rule_id, toggle) in &self.rule_toggles {
            let Some(waiver) = &toggle.waiver else {
                if matches!(toggle.enabled, RuleEnabled::Disabled) {
                    return Err(PolicyValidationError::DisabledRuleWithoutWaiver);
                }
                continue;
            };
            if waiver.rule_id != *rule_id {
                return Err(PolicyValidationError::WaiverRuleIdMismatch);
            }
        }
        Ok(())
    }

    /// Whether `rule_id` is enabled under this policy. Absence of a toggle
    /// entry means enabled (the default), never disabled by omission.
    pub fn rule_enabled(&self, rule_id: &RuleId) -> RuleEnabled {
        self.rule_toggles
            .get(rule_id)
            .map(|toggle| toggle.enabled)
            .unwrap_or_else(RuleEnabled::enabled)
    }

    /// The effective severity for `rule_id` given `default_severity`: an
    /// enabled toggle's `severity` override wins, otherwise the default.
    pub fn effective_severity(
        &self,
        rule_id: &RuleId,
        default_severity: enforcer_domain::severity::Severity,
    ) -> enforcer_domain::severity::Severity {
        self.rule_toggles
            .get(rule_id)
            .filter(|toggle| matches!(toggle.enabled, RuleEnabled::Enabled))
            .and_then(|toggle| toggle.severity)
            .unwrap_or(default_severity)
    }
}
