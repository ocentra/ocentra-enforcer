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

use std::collections::BTreeMap;

use enforcer_domain::ids::RuleId;
use serde::{Deserialize, Serialize};

use crate::model::Glob;

/// A single per-rule override: enable/disable, severity override, and/or a
/// waiver. Disabling a rule outright still requires [`Waiver`] fields
/// (owner + reason) per the honesty doctrine — there is no bare `false`
/// escape hatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleToggle {
    /// Whether the rule is enabled. Defaults to `true`: absence of a toggle
    /// entry never disables a rule.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional severity override for this rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<enforcer_domain::severity::Severity>,
    /// Required when `enabled = false`: the attributable waiver record.
    /// Absence while `enabled = false` is a boundary error (see
    /// [`crate::error::ConfigLoadError`]), not a silent disable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver: Option<Waiver>,
}

fn default_true() -> bool {
    true
}

/// An attributable waiver: who granted the exception and why. Required to
/// disable a rule; there is no anonymous or reasonless disable path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Waiver {
    /// The rule this waiver applies to (redundant with the map key, kept for
    /// self-description when a waiver record is extracted/logged alone).
    pub rule_id: RuleId,
    /// Who owns/granted this waiver (person or team handle, free text).
    pub owner: String,
    /// Why the rule is waived for this project. Must be non-empty (checked
    /// at boundary).
    pub reason: String,
}

/// The full declarative policy externalization surface. Every field here is
/// data read from `.enforce/config`; there is no code path that lets a
/// target source file locally suppress a rule instead.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    /// Globs marking files whose owning team/individual is committed policy
    /// (not tribal knowledge).
    #[serde(default)]
    pub owner_globs: Vec<Glob>,
    /// Globs exempt from enforcement entirely (e.g. generated code,
    /// vendored sources). Distinct from a rule-level waiver: this exempts
    /// the *path*, not a *rule*.
    #[serde(default)]
    pub exempt_globs: Vec<Glob>,
    /// Regex source patterns (escaped) that, when a line/path matches, are
    /// allowed despite an otherwise-matching rule.
    #[serde(default)]
    pub allow_regex: Vec<String>,
    /// Whether `#[cfg(test)]` modules and conventional test-path globs are
    /// skipped by scope-sensitive rules.
    #[serde(default)]
    pub skip_cfg_test: bool,
    /// Additional test-path globs skipped alongside `#[cfg(test)]` (e.g.
    /// `tests/**`, `**/*_test.rs`) when [`Policy::skip_cfg_test`] is set.
    #[serde(default)]
    pub test_path_globs: Vec<Glob>,
    /// Per-rule toggles, keyed by [`RuleId`]. Absence of a key means the
    /// rule runs at its default severity — never silently disabled by
    /// omission.
    #[serde(default)]
    pub rule_toggles: BTreeMap<RuleId, RuleToggle>,
}

impl Policy {
    /// Validate cross-field invariants that plain `serde` shape checking
    /// cannot express: a disabled rule must carry a non-empty waiver
    /// (owner + reason), matching the honesty doctrine — no silent
    /// suppression.
    ///
    /// # Errors
    /// Returns a description of the first invariant violation found.
    pub fn validate(&self) -> Result<(), String> {
        for (rule_id, toggle) in &self.rule_toggles {
            if toggle.enabled {
                continue;
            }
            let Some(waiver) = &toggle.waiver else {
                return Err(format!(
                    "rule `{rule_id}` is disabled but carries no waiver (owner + reason required; inline/silent disables are banned)"
                ));
            };
            if waiver.owner.trim().is_empty() {
                return Err(format!("waiver for rule `{rule_id}` has an empty owner"));
            }
            if waiver.reason.trim().is_empty() {
                return Err(format!("waiver for rule `{rule_id}` has an empty reason"));
            }
            if waiver.rule_id != *rule_id {
                return Err(format!(
                    "waiver.ruleId `{}` does not match its map key `{rule_id}`",
                    waiver.rule_id
                ));
            }
        }
        Ok(())
    }

    /// Whether `rule_id` is enabled under this policy. Absence of a toggle
    /// entry means enabled (the default), never disabled by omission.
    pub fn is_rule_enabled(&self, rule_id: &RuleId) -> bool {
        self.rule_toggles
            .get(rule_id)
            .map(|toggle| toggle.enabled)
            .unwrap_or(true)
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
            .and_then(|toggle| toggle.severity)
            .unwrap_or(default_severity)
    }
}

#[cfg(test)]
mod tests {
    use super::{Policy, RuleToggle, Waiver};
    use enforcer_domain::ids::RuleId;
    use enforcer_domain::severity::Severity;
    use std::collections::BTreeMap;
    use std::str::FromStr;

    /// Test-only helper: parse a known-valid rule id literal, propagating a
    /// parse failure via `Result` (no `unwrap`/`expect`, both denied by
    /// workspace clippy lints) so a typo in a fixture surfaces as a failed
    /// test, not a panic.
    fn rule_id(s: &str) -> Result<RuleId, enforcer_core::error::DecodeError> {
        RuleId::from_str(s)
    }

    #[test]
    fn absent_toggle_means_enabled() -> Result<(), Box<dyn std::error::Error>> {
        let policy = Policy::default();
        assert!(policy.is_rule_enabled(&rule_id("RR-1.1")?));
        Ok(())
    }

    #[test]
    fn disabled_rule_without_waiver_fails_validation() -> Result<(), Box<dyn std::error::Error>> {
        let mut toggles = BTreeMap::new();
        toggles.insert(
            rule_id("RR-1.1")?,
            RuleToggle {
                enabled: false,
                severity: None,
                waiver: None,
            },
        );
        let policy = Policy {
            rule_toggles: toggles,
            ..Policy::default()
        };
        assert!(policy.validate().is_err());
        Ok(())
    }

    #[test]
    fn disabled_rule_with_waiver_passes_validation() -> Result<(), Box<dyn std::error::Error>> {
        let mut toggles = BTreeMap::new();
        toggles.insert(
            rule_id("RR-1.1")?,
            RuleToggle {
                enabled: false,
                severity: None,
                waiver: Some(Waiver {
                    rule_id: rule_id("RR-1.1")?,
                    owner: "platform-team".to_owned(),
                    reason: "legacy module pending migration".to_owned(),
                }),
            },
        );
        let policy = Policy {
            rule_toggles: toggles,
            ..Policy::default()
        };
        assert!(policy.validate().is_ok());
        assert!(!policy.is_rule_enabled(&rule_id("RR-1.1")?));
        Ok(())
    }

    #[test]
    fn waiver_rule_id_mismatch_fails_validation() -> Result<(), Box<dyn std::error::Error>> {
        let mut toggles = BTreeMap::new();
        toggles.insert(
            rule_id("RR-1.1")?,
            RuleToggle {
                enabled: false,
                severity: None,
                waiver: Some(Waiver {
                    rule_id: rule_id("RR-2.2")?,
                    owner: "team".to_owned(),
                    reason: "reason".to_owned(),
                }),
            },
        );
        let policy = Policy {
            rule_toggles: toggles,
            ..Policy::default()
        };
        assert!(policy.validate().is_err());
        Ok(())
    }

    #[test]
    fn severity_override_wins_over_default() -> Result<(), Box<dyn std::error::Error>> {
        let mut toggles = BTreeMap::new();
        toggles.insert(
            rule_id("RR-1.1")?,
            RuleToggle {
                enabled: true,
                severity: Some(Severity::Warning),
                waiver: None,
            },
        );
        let policy = Policy {
            rule_toggles: toggles,
            ..Policy::default()
        };
        assert_eq!(
            policy.effective_severity(&rule_id("RR-1.1")?, Severity::Error),
            Severity::Warning
        );
        assert_eq!(
            policy.effective_severity(&rule_id("RR-9.9")?, Severity::Error),
            Severity::Error
        );
        Ok(())
    }

    #[test]
    fn policy_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>> {
        let mut toggles = BTreeMap::new();
        toggles.insert(
            rule_id("SEC-1.1")?,
            RuleToggle {
                enabled: false,
                severity: None,
                waiver: Some(Waiver {
                    rule_id: rule_id("SEC-1.1")?,
                    owner: "sec-team".to_owned(),
                    reason: "tracked in TICKET-123".to_owned(),
                }),
            },
        );
        let policy = Policy {
            rule_toggles: toggles,
            skip_cfg_test: true,
            ..Policy::default()
        };
        let wire = serde_json::to_string(&policy)?;
        let back: Policy = serde_json::from_str(&wire)?;
        assert_eq!(back, policy);
        Ok(())
    }
}
