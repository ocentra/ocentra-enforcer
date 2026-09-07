//! Typed inspection data for governance checks at the JSON ingress boundary.
//!
//! This is deliberately separate from normal resolution: [`crate::load_project_config`]
//! remains fail-closed.  The governance check needs to report each invalid
//! project setting instead of losing it at the serde boundary, so it decodes a
//! small, private read model and exposes only typed diagnostic records.

use enforcer_domain::config_types::EffectiveConfig;
use std::collections::BTreeMap;
use std::path::Path;

const KNOWN_TOP_LEVEL_KEYS: &[&str] = &[
    "schemaVersion",
    "profileName",
    "failOn",
    "failFast",
    "enforceWorkspaceFiles",
    "requireCargoDeny",
    "requireCargoAudit",
    "runCargoDoc",
    "cargoOnFileScope",
    "cargoOnDiffScope",
    "cargoTestThreads",
    "allowUnsafeCode",
    "allowBuildRs",
    "allowedBuildRsPaths",
    "allowGitDependencies",
    "allowPathDependencies",
    "publicReexportPolicy",
    "ignoreDirs",
    "ignoreFileGlobs",
    "rustRoots",
    "crateRootGlobs",
    "testFileGlobs",
    "rawTypeBoundaryGlobs",
    "boundaryOwnerNote",
    "facadeFileGlobs",
    "rawStringOwnerGlobs",
    "domainPrimitiveOwnerGlobs",
    "enforceRuntimeStringLiterals",
    "runtimeStringOwnerGlobs",
    "runtimeStringLineAllowPatterns",
    "enforceSerializedPublicDomainPrimitives",
    "serializedDomainOwnerGlobs",
    "blockedProtocolDependencies",
    "runtimeCrates",
    "testOnlyCrates",
    "allowedGitDependencies",
    "allowedExternalLicenses",
    "sourceShapePolicies",
    "sourceShapeOverrides",
    "importBoundaryPolicies",
    "architecturePolicyChecks",
    "singleSourceRequiredMirrorRoots",
    "strictEmptyTestTrees",
    "privateRustTestModuleAllowlist",
    "generatedArtifactsMode",
    "generatedArtifactsTracked",
    "generatedArtifactsAllowlist",
    "agentRuleMaxLines",
    "maxActiveWaivers",
    "maxWaiverDays",
    "configChangeRequiresSelfCheck",
    "policyIntegrityChecked",
    "languages",
    "rules",
    "waivers",
    "tools",
    "harness",
];

const BOUNDARY_CONFIG_KEYS: &[&str] = &[
    "rawTypeBoundaryGlobs",
    "facadeFileGlobs",
    "rawStringOwnerGlobs",
    "domainPrimitiveOwnerGlobs",
    "runtimeStringOwnerGlobs",
    "runtimeStringLineAllowPatterns",
    "serializedDomainOwnerGlobs",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleOverrideDiagnostic {
    pub enabled: Option<bool>,
    pub severity: Option<String>,
    pub has_complete_disable_waiver: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaiverDiagnostic {
    pub rule_id: Option<String>,
    pub waiver_id: Option<String>,
    pub owner: Option<String>,
    pub issue: Option<String>,
    pub reason: Option<String>,
    pub expires: Option<String>,
    pub remediation: Option<String>,
    pub scope: Option<Vec<String>>,
    pub ci_allowed: Option<bool>,
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobPolicyDiagnostic {
    pub has_glob: bool,
    pub has_note: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigParseDiagnostics {
    pub effective: Option<EffectiveConfig>,
    pub unknown_top_level_keys: Vec<String>,
    pub missing_schema_version: bool,
    pub missing_profile_name: bool,
    pub profile_name: Option<String>,
    pub load_error: Option<String>,
    pub fail_on: Vec<String>,
    pub config_change_requires_self_check: bool,
    pub policy_integrity_checked: bool,
    pub rules: BTreeMap<String, RuleOverrideDiagnostic>,
    pub boundary_fields_without_owner_note: Vec<String>,
    pub source_shape_overrides: Vec<GlobPolicyDiagnostic>,
    pub import_boundary_policies: Vec<GlobPolicyDiagnostic>,
    pub allow_unsafe_code: bool,
    pub public_reexport_policy: Option<String>,
    pub allow_build_rs: bool,
    pub allow_git_dependencies: bool,
    pub allow_path_dependencies: bool,
    pub waivers: Vec<WaiverDiagnostic>,
    pub max_active_waivers: Option<usize>,
    pub max_waiver_days: usize,
}

fn string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(serde_json::Value::as_str).map(str::to_owned)
}
fn bool_value(value: Option<&serde_json::Value>) -> bool {
    value.and_then(serde_json::Value::as_bool).unwrap_or(false)
}
fn nonempty(value: Option<&serde_json::Value>) -> Option<String> {
    string(value).filter(|value| !value.trim().is_empty())
}
fn string_array(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value.and_then(serde_json::Value::as_array).map(|values| {
        values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect()
    })
}

fn glob_policies(value: Option<&serde_json::Value>) -> Vec<GlobPolicyDiagnostic> {
    value
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_object)
                .map(|entry| {
                    let has_glob = entry
                        .get("glob")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|v| !v.trim().is_empty())
                        || entry
                            .get("globs")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|v| !v.is_empty());
                    GlobPolicyDiagnostic {
                        has_glob,
                        has_note: nonempty(entry.get("note")).is_some(),
                    }
                })
                .collect()
        })
}

fn waiver(value: &serde_json::Value) -> WaiverDiagnostic {
    let object = value.as_object();
    WaiverDiagnostic {
        rule_id: nonempty(object.and_then(|v| v.get("ruleId"))),
        waiver_id: nonempty(object.and_then(|v| v.get("waiverId"))),
        owner: nonempty(object.and_then(|v| v.get("owner"))),
        issue: nonempty(object.and_then(|v| v.get("issue"))),
        reason: nonempty(object.and_then(|v| v.get("reason"))),
        expires: nonempty(object.and_then(|v| v.get("expires"))),
        remediation: nonempty(object.and_then(|v| v.get("remediation"))),
        scope: string_array(object.and_then(|v| v.get("scope"))),
        ci_allowed: object
            .and_then(|v| v.get("ciAllowed"))
            .and_then(serde_json::Value::as_bool),
        visible: object
            .and_then(|v| v.get("visible"))
            .and_then(serde_json::Value::as_bool),
    }
}

pub fn inspect_project_config(path: &Path) -> ConfigParseDiagnostics {
    let raw = std::fs::read_to_string(path);
    let value = raw
        .as_ref()
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok());
    let object = value.as_ref().and_then(serde_json::Value::as_object);
    let mut unknown_top_level_keys: Vec<String> = object
        .map(|object| {
            object
                .keys()
                .filter(|key| !KNOWN_TOP_LEVEL_KEYS.contains(&key.as_str()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    unknown_top_level_keys.sort();
    let missing_schema_version = object.is_some_and(|object| !object.contains_key("schemaVersion"));
    let missing_profile_name = object.is_some_and(|object| !object.contains_key("profileName"));
    let rules = object
        .and_then(|value| value.get("rules"))
        .and_then(serde_json::Value::as_object)
        .map_or_else(BTreeMap::new, |entries| {
            entries
                .iter()
                .map(|(id, value)| {
                    let entry = value.as_object();
                    let complete = [
                        "waiverId",
                        "owner",
                        "issue",
                        "reason",
                        "expires",
                        "remediation",
                    ]
                    .iter()
                    .all(|key| nonempty(entry.and_then(|v| v.get(*key))).is_some())
                        && string_array(entry.and_then(|v| v.get("scope")))
                            .is_some_and(|scope| !scope.is_empty());
                    (
                        id.clone(),
                        RuleOverrideDiagnostic {
                            enabled: entry
                                .and_then(|v| v.get("enabled"))
                                .and_then(serde_json::Value::as_bool),
                            severity: string(entry.and_then(|v| v.get("severity"))),
                            has_complete_disable_waiver: complete,
                        },
                    )
                })
                .collect()
        });
    let boundary_owner_note =
        nonempty(object.and_then(|value| value.get("boundaryOwnerNote"))).is_some();
    let boundary_fields_without_owner_note = object
        .map(|entries| {
            BOUNDARY_CONFIG_KEYS
                .iter()
                .filter(|field| {
                    entries
                        .get(**field)
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|values| !values.is_empty())
                        && !boundary_owner_note
                })
                .map(|field| (*field).to_owned())
                .collect()
        })
        .unwrap_or_default();
    let waivers = object
        .and_then(|value| value.get("waivers"))
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |values| values.iter().map(waiver).collect());
    let effective = crate::load_project_config(path).ok();
    let load_error = crate::load_project_config(path)
        .err()
        .map(|error| error.to_string());
    ConfigParseDiagnostics {
        effective,
        unknown_top_level_keys,
        missing_schema_version,
        missing_profile_name,
        profile_name: string(object.and_then(|value| value.get("profileName"))),
        load_error,
        fail_on: string_array(object.and_then(|value| value.get("failOn"))).unwrap_or_default(),
        config_change_requires_self_check: bool_value(
            object.and_then(|value| value.get("configChangeRequiresSelfCheck")),
        ),
        policy_integrity_checked: object
            .and_then(|value| value.get("policyIntegrityChecked"))
            .and_then(serde_json::Value::as_bool)
            == Some(true),
        rules,
        boundary_fields_without_owner_note,
        source_shape_overrides: glob_policies(
            object.and_then(|value| value.get("sourceShapeOverrides")),
        ),
        import_boundary_policies: glob_policies(
            object.and_then(|value| value.get("importBoundaryPolicies")),
        ),
        allow_unsafe_code: bool_value(object.and_then(|value| value.get("allowUnsafeCode"))),
        public_reexport_policy: string(object.and_then(|value| value.get("publicReexportPolicy"))),
        allow_build_rs: bool_value(object.and_then(|value| value.get("allowBuildRs"))),
        allow_git_dependencies: bool_value(
            object.and_then(|value| value.get("allowGitDependencies")),
        ),
        allow_path_dependencies: bool_value(
            object.and_then(|value| value.get("allowPathDependencies")),
        ),
        waivers,
        max_active_waivers: object
            .and_then(|value| value.get("maxActiveWaivers"))
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok()),
        max_waiver_days: object
            .and_then(|value| value.get("maxWaiverDays"))
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(90),
    }
}

#[cfg(test)]
mod tests {
    use super::inspect_project_config;
    #[test]
    fn reports_governance_data_without_weakening_normal_load(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "enforcer-config-diagnostics-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"{"unknown":true,"rules":{"RR-1.1":{"enabled":false}},"waivers":[{}]}"#,
        )?;
        let diagnostics = inspect_project_config(&path);
        std::fs::remove_file(path)?;
        assert_eq!(diagnostics.unknown_top_level_keys, vec!["unknown"]);
        assert!(diagnostics.missing_schema_version && diagnostics.missing_profile_name);
        assert!(diagnostics.effective.is_none());
        assert_eq!(diagnostics.rules.len(), 1);
        assert_eq!(diagnostics.waivers.len(), 1);
        Ok(())
    }
}
