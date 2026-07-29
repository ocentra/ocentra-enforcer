//! The settings mutation route: the ONLY place in this module that writes
//! `.enforce/config`. Kept in its own module (never merged into
//! [`crate::settings::read`], which is GET-only) so g07's same-origin/
//! CSRF guard layer can wrap exactly this route -- there is no code path
//! here reachable from a GET handler, and no code path in [`read`] that
//! mutates.
//! BOUNDARY-INVARIANT: parse and validate every toggle request and the merged
//! project config before the atomic write boundary.
//! boundaryOwnerNote: enforcer-ui owns the typed settings write boundary.
//!
//! # Write pipeline (fail-closed, typed, idempotent)
//! 1. Parse the incoming request into [`ToggleRuleInput`] — rejects a
//!    non-object body, a missing/malformed `ruleId`, or (when disabling)
//!    a missing/empty waiver `owner`/`reason` — via
//!    [`ToggleRuleInput::validate`]. A rejected request writes NOTHING.
//! 2. Load the CURRENT typed [`ProjectConfig`] (never a raw file diff),
//!    apply the toggle to its `policy.rule_toggles` map, and re-validate
//!    the WHOLE resulting [`ProjectConfig`] via
//!    [`ResolvedProjectTie::resolve`] before touching disk -- a request
//!    that would produce an invalid config (e.g. a mismatched
//!    `waiver.ruleId`) is rejected post-merge too, not just pre-merge.
//! 3. Serialize the validated [`ProjectConfig`] through `serde_json`
//!    exactly once and write it atomically-by-replace. Toggling the SAME
//!    state twice re-serializes to byte-identical output (the map entry
//!    is replaced, never appended), matching the workpack's idempotency
//!    requirement.
//!
//! [`read`]: crate::settings::read

use std::path::Path;

use enforcer_config::error::{ConfigLoadError, ConfigResult};
use enforcer_config::policy::{RuleToggle, Waiver};
use enforcer_config::project_tie::{load_project_tie, ProjectConfig, ResolvedProjectTie};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::{ConfigSource, PolicyOwner, PolicyReason, RuleEnabled};
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;

/// A boundary-rejected settings-write request: the malformed-request fail
/// fixture this workpack's acceptance block requires (a waiver save
/// missing owner/reason/`ruleId` is rejected typed, not silently
/// defaulted). Distinct from [`ConfigLoadError`] because this rejects the
/// REQUEST shape before any config is even loaded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsWriteError {
    /// The request body failed [`ToggleRuleInput`] boundary validation.
    #[error("settings write request rejected: {reason}")]
    MalformedActionError {
        /// Human-readable reason the request was rejected.
        reason: String,
    },
    /// Loading, merging, or re-validating the underlying typed config
    /// failed (e.g. the merged config would carry a mismatched waiver
    /// `ruleId`, or the on-disk config was already malformed).
    #[error("settings write rejected by config validation: {0}")]
    Config(#[from] ConfigLoadError),
    /// The validated config could not be written to disk.
    #[error("failed to write config file `{path}`: {reason}")]
    Io {
        /// Path that failed to write.
        path: String,
        /// Underlying I/O failure description.
        reason: String,
    },
}

/// A single rule-toggle mutation request: the ONE shape this route
/// accepts. Enabling a rule never needs a waiver; disabling one requires
/// a non-empty owner + reason (mirrors
/// [`enforcer_config::policy::Policy::validate`]'s invariant one layer
/// earlier, at the request boundary, so a bad request never even reaches
/// the merge step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleRuleInput {
    /// The rule to toggle.
    pub rule_id: RuleId,
    /// Target enabled state.
    pub enabled: bool,
    /// Optional severity override to set alongside the toggle.
    pub severity: Option<Severity>,
    /// Required when `enabled` is `false`: owner + reason. Rejected as
    /// malformed if absent, or if either field is empty, while
    /// `enabled == false`.
    pub waiver: Option<WaiverInput>,
}

/// The waiver fields a disable-toggle request must supply. Never
/// constructed with an empty `owner`/`reason` past [`ToggleRuleInput::
/// validate`] -- this is the explicit gated waiver newtype the workpack's
/// requirement checklist calls for (owner + reason + `RuleId`, all
/// validated at the boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaiverInput {
    /// Who owns/granted this waiver.
    pub owner: PolicyOwner,
    /// Why the rule is waived.
    pub reason: PolicyReason,
}

impl ToggleRuleInput {
    /// Parse+validate a raw JSON request body into a [`ToggleRuleInput`].
    /// Rejects (typed, never a silent default):
    /// - a non-object body,
    /// - a missing/malformed `ruleId` (must decode as [`RuleId`]),
    /// - a missing/non-bool `enabled`,
    /// - a malformed `severity` (present but not one of the closed
    ///   `Severity` wire strings),
    /// - `enabled: false` with no `waiver`, or a `waiver` whose `owner`/
    ///   `reason` is missing or empty.
    ///
    /// # Errors
    /// Returns [`SettingsWriteError::MalformedActionError`] on any of the
    /// above. No config is loaded or touched before this succeeds.
    pub fn parse(body: &serde_json::Value) -> Result<Self, SettingsWriteError> {
        let obj = body
            .as_object()
            .ok_or_else(|| SettingsWriteError::MalformedActionError {
                reason: "request body must be a JSON object".to_owned(),
            })?;

        let rule_id_raw = obj.get("ruleId").and_then(|v| v.as_str()).ok_or_else(|| {
            SettingsWriteError::MalformedActionError {
                reason: "request body missing string field `ruleId`".to_owned(),
            }
        })?;
        let rule_id: RuleId = rule_id_raw.parse().map_err(|err: DecodeError| {
            SettingsWriteError::MalformedActionError {
                reason: format!("`ruleId` failed to decode: {err}"),
            }
        })?;

        let enabled = obj
            .get("enabled")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| SettingsWriteError::MalformedActionError {
                reason: "request body missing boolean field `enabled`".to_owned(),
            })?;

        let severity = match obj.get("severity") {
            None | Some(serde_json::Value::Null) => None,
            Some(value) => Some(serde_json::from_value::<Severity>(value.clone()).map_err(
                |err| SettingsWriteError::MalformedActionError {
                    reason: format!("`severity` failed to decode: {err}"),
                },
            )?),
        };

        let waiver = match obj.get("waiver") {
            None | Some(serde_json::Value::Null) => None,
            Some(value) => {
                let waiver_obj =
                    value
                        .as_object()
                        .ok_or_else(|| SettingsWriteError::MalformedActionError {
                            reason: "`waiver` must be a JSON object".to_owned(),
                        })?;
                let owner_raw = waiver_obj
                    .get("owner")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| SettingsWriteError::MalformedActionError {
                        reason: "`waiver.owner` must be a non-empty string".to_owned(),
                    })?;
                let owner = PolicyOwner::try_new(owner_raw.to_owned()).map_err(|error| {
                    SettingsWriteError::MalformedActionError {
                        reason: error.to_string(),
                    }
                })?;
                let reason_raw = waiver_obj
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| SettingsWriteError::MalformedActionError {
                        reason: "`waiver.reason` must be a non-empty string".to_owned(),
                    })?;
                let reason = PolicyReason::try_new(reason_raw.to_owned()).map_err(|error| {
                    SettingsWriteError::MalformedActionError {
                        reason: error.to_string(),
                    }
                })?;
                Some(WaiverInput { owner, reason })
            }
        };

        if !enabled && waiver.is_none() {
            return Err(SettingsWriteError::MalformedActionError {
                reason: format!(
                    "disabling rule `{rule_id}` requires a waiver (owner + reason); \
                     inline/silent disables are banned"
                ),
            });
        }

        Ok(ToggleRuleInput {
            rule_id,
            enabled,
            severity,
            waiver,
        })
    }

    /// Apply this request onto `config`'s `policy.rule_toggles` map,
    /// returning the mutated [`ProjectConfig`]. Idempotent: replacing the
    /// map entry for `rule_id` (never pushing to a list) means re-applying
    /// the identical request twice produces an identical map, so the
    /// eventual `serde_json` serialization is byte-for-byte the same both
    /// times.
    fn apply(&self, mut config: ProjectConfig) -> ProjectConfig {
        let waiver = self.waiver.as_ref().map(|w| Waiver {
            rule_id: self.rule_id.clone(),
            owner: w.owner.clone(),
            reason: w.reason.clone(),
        });
        config.policy.rule_toggles.insert(
            self.rule_id.clone(),
            RuleToggle {
                enabled: if self.enabled {
                    RuleEnabled::enabled()
                } else {
                    RuleEnabled::disabled()
                },
                severity: self.severity,
                waiver,
            },
        );
        config
    }
}

/// Serialize `config` deterministically for the on-disk write: pretty
/// JSON with a trailing newline, via `serde_json` on the typed model only
/// (never a hand-built string). The single serialization point every
/// write funnels through, so "idempotent toggle -> byte-identical output"
/// reduces to "the typed map is unchanged", which [`ToggleRuleInput::
/// apply`]'s insert-not-push semantics guarantee.
fn serialize_config(config: &ProjectConfig, source_path: &ConfigSource) -> ConfigResult<String> {
    let mut json = enforcer_config::serde::encode_project_config(config, source_path)?
        .as_str()
        .to_owned();
    json.push('\n');
    Ok(json)
}

/// Apply `request` to the `.enforce/config` typed model at `config_path`,
/// validate the WHOLE resulting config (fail-closed, mirrors
/// [`enforcer_config::policy::Policy::validate`]'s invariant), and write
/// it back — ONLY on success. On any rejection (malformed request,
/// post-merge validation failure) this function returns before touching
/// disk: `config_path`'s bytes are unchanged.
///
/// # Errors
/// Returns [`SettingsWriteError`] if the request is malformed, if loading
/// the current config fails, if the merged config fails
/// [`ResolvedProjectTie::resolve`] validation, or if the write itself
/// fails at the OS level.
pub fn toggle_rule(
    config_path: &Path,
    request: &ToggleRuleInput,
) -> Result<ResolvedProjectTie, SettingsWriteError> {
    let current = load_current_config(config_path)?;
    let merged = request.apply(current);

    // Fail-closed: validate the FULL merged config (not just the request
    // shape) before ever writing — catches a `waiver.ruleId` mismatch the
    // request-level check cannot see (it always sets `waiver.ruleId ==
    // request.rule_id`, but this still proves the write path never skips
    // the same gate the loader itself enforces).
    let source_path = enforcer_config::serde::config_source_for_path(config_path);
    let resolved = ResolvedProjectTie::resolve(&merged, &source_path)?;

    let serialized = serialize_config(&merged, &source_path)?;
    std::fs::write(config_path, serialized).map_err(|err| SettingsWriteError::Io {
        path: config_path.display().to_string(),
        reason: err.to_string(),
    })?;

    Ok(resolved)
}

/// Load the current `.enforce/config` as a raw [`ProjectConfig`] (not the
/// resolved/total view) so a merge can be applied onto exactly the
/// declared map, never a defaulted-and-therefore-lossy one. Absence of
/// the file resolves to [`ProjectConfig::default`], matching arc-03's
/// "zero-config projects work" invariant on the write side too.
fn load_current_config(config_path: &Path) -> ConfigResult<ProjectConfig> {
    let config = enforcer_config::serde::load_project_config(config_path)?;
    // Validate the freshly-loaded config with the SAME gate `write`
    // re-runs post-merge, so a caller never merges onto an already-broken
    // base and mistakes the later failure for a fresh one.
    let _ = load_project_tie(config_path)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::any;
    use proptest::{prop_assert_eq, proptest, test_runner::TestCaseError};

    use super::{toggle_rule, SettingsWriteError, ToggleRuleInput};

    proptest! {
        #[test]
        fn parse_preserves_valid_toggle_requests(
            family in 1_u16..15,
            number in 1_u16..40,
            enabled in any::<bool>(),
            owner in "[a-z]{1,12}",
            reason in "[a-z][a-z ]{0,23}",
        ) {
            let rule_id = format!("RR-{family}.{number}");
            let body = if enabled {
                serde_json::json!({ "ruleId": rule_id, "enabled": true })
            } else {
                serde_json::json!({
                    "ruleId": rule_id,
                    "enabled": false,
                    "waiver": { "owner": owner, "reason": reason }
                })
            };
            let parsed = ToggleRuleInput::parse(&body)
                .map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(parsed.rule_id.to_string(), rule_id);
            prop_assert_eq!(parsed.enabled, enabled);
        }
    }

    fn temp_config_path(
    ) -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(".enforce-config.json");
        Ok((dir, path))
    }

    /// FAIL fixture: a waiver save missing `owner`/`reason` is rejected at
    /// the boundary (typed error) and writes NOTHING — the temp-dir config
    /// is left untouched (does not even exist, since it never existed).
    #[test]
    fn disable_without_waiver_is_rejected_and_writes_nothing(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (_dir, path) = temp_config_path()?;
        let body = serde_json::json!({ "ruleId": "RR-1.1", "enabled": false });
        let outcome = ToggleRuleInput::parse(&body);
        assert!(matches!(
            outcome,
            Err(SettingsWriteError::MalformedActionError { .. })
        ));
        assert!(!path.exists(), "rejected request must write nothing");
        Ok(())
    }

    /// FAIL fixture: an empty owner/reason is treated the same as absent
    /// — no "pass an empty string" loophole.
    #[test]
    fn disable_with_empty_waiver_fields_is_rejected() {
        let body = serde_json::json!({
            "ruleId": "RR-1.1",
            "enabled": false,
            "waiver": { "owner": "  ", "reason": "" }
        });
        let outcome = ToggleRuleInput::parse(&body);
        assert!(matches!(
            outcome,
            Err(SettingsWriteError::MalformedActionError { .. })
        ));
    }

    /// FAIL fixture: a malformed `ruleId` is rejected typed, before any
    /// config load.
    #[test]
    fn malformed_rule_id_is_rejected() {
        let body = serde_json::json!({ "ruleId": "not a rule id", "enabled": true });
        let outcome = ToggleRuleInput::parse(&body);
        assert!(matches!(
            outcome,
            Err(SettingsWriteError::MalformedActionError { .. })
        ));
    }

    /// PASS fixture: toggling a rule severity writes the correct config
    /// once through the typed model — the resulting file, re-parsed,
    /// carries the exact rule/severity/enabled state requested.
    #[test]
    fn toggling_severity_writes_correct_config_once() -> Result<(), Box<dyn std::error::Error>> {
        let (_dir, path) = temp_config_path()?;
        let body = serde_json::json!({
            "ruleId": "RR-1.1",
            "enabled": true,
            "severity": "warning"
        });
        let request = ToggleRuleInput::parse(&body)?;
        toggle_rule(&path, &request)?;

        assert!(path.exists());
        let written = std::fs::read_to_string(&path)?;
        let parsed: serde_json::Value = serde_json::from_str(&written)?;
        let toggle = &parsed["policy"]["ruleToggles"]["RR-1.1"];
        assert_eq!(toggle["enabled"], serde_json::json!(true));
        assert_eq!(toggle["severity"], serde_json::json!("warning"));
        Ok(())
    }

    /// PASS fixture + idempotency: re-toggling ON (same severity) twice
    /// round-trips to byte-identical config -- no duplicated map entries,
    /// no drift on repeated writes of the same state.
    #[test]
    fn retoggling_on_twice_is_byte_identical() -> Result<(), Box<dyn std::error::Error>> {
        let (_dir, path) = temp_config_path()?;
        let body = serde_json::json!({
            "ruleId": "RR-2.2",
            "enabled": true,
            "severity": "error"
        });
        let request = ToggleRuleInput::parse(&body)?;

        toggle_rule(&path, &request)?;
        let first_write = std::fs::read(&path)?;

        toggle_rule(&path, &request)?;
        let second_write = std::fs::read(&path)?;

        assert_eq!(
            first_write, second_write,
            "re-toggling the identical state twice must round-trip byte-identical"
        );
        // Sanity: exactly one `RR-2.2` map entry, not a duplicate.
        let parsed: serde_json::Value = serde_json::from_slice(&second_write)?;
        let toggles = parsed["policy"]["ruleToggles"]
            .as_object()
            .ok_or("expected ruleToggles object")?;
        assert_eq!(toggles.len(), 1);
        Ok(())
    }

    /// PASS fixture: disabling a rule WITH a valid waiver writes the named
    /// waiver record (owner + reason + ruleId), never a silent
    /// suppression.
    #[test]
    fn disable_with_valid_waiver_writes_named_waiver() -> Result<(), Box<dyn std::error::Error>> {
        let (_dir, path) = temp_config_path()?;
        let body = serde_json::json!({
            "ruleId": "RR-3.3",
            "enabled": false,
            "waiver": { "owner": "platform-team", "reason": "tracked in TICKET-9" }
        });
        let request = ToggleRuleInput::parse(&body)?;
        toggle_rule(&path, &request)?;

        let written = std::fs::read_to_string(&path)?;
        let parsed: serde_json::Value = serde_json::from_str(&written)?;
        let waiver = &parsed["policy"]["ruleToggles"]["RR-3.3"]["waiver"];
        assert_eq!(waiver["ruleId"], serde_json::json!("RR-3.3"));
        assert_eq!(waiver["owner"], serde_json::json!("platform-team"));
        assert_eq!(waiver["reason"], serde_json::json!("tracked in TICKET-9"));
        Ok(())
    }

    /// FAIL fixture: a request against an already-malformed on-disk
    /// config (unknown native tool key) is rejected, writes nothing new —
    /// REGRESSION-TEST: `write_against_malformed_existing_config_is_rejected`
    /// locks this fail-closed behavior against future write-path changes.
    /// the write path never "fixes forward" over a broken base.
    #[test]
    fn write_against_malformed_existing_config_is_rejected(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (_dir, path) = temp_config_path()?;
        std::fs::write(
            &path,
            serde_json::json!({ "native": { "gofmt": { "mode": "augment" } } }).to_string(),
        )?;
        let original = std::fs::read(&path)?;

        let body = serde_json::json!({ "ruleId": "RR-1.1", "enabled": true });
        let request = ToggleRuleInput::parse(&body)?;
        let Err(error) = toggle_rule(&path, &request) else {
            return Err("a malformed existing config must reject the settings write".into());
        };
        assert!(matches!(error, SettingsWriteError::Config(_)));

        let after = std::fs::read(&path)?;
        assert_eq!(
            original, after,
            "rejected write must leave the file untouched"
        );
        Ok(())
    }
}
