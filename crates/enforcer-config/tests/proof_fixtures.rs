//! Configuration resolution proof fixtures.
//!
//! Proof fixtures for the arc-03 workpack acceptance criteria: 3-layer
//! resolution cases, per-field-group `EffectiveConfig` round-trips
//! [G5], the `sourceShapePolicies` base-shape fixture [G4], the
//! `ocentra-parent`-posture resolution fixture, and the cover-all
//! completeness gate (every JSON key in the three real config files
//! maps to a typed field).

use enforcer_config::error::ConfigLoadError;
use enforcer_config::resolve::{resolve, resolve_profile_only};
use enforcer_config::serde::{decode_json, WireEffectiveConfig};
use enforcer_domain::config_types::{
    ConfigJson, ConfigProfileName, ConfigSource, EffectiveConfig, Platform, PublicReexportPolicy,
    RuleEnabled, SourceShapeKind,
};
use serde_json::{json, Value};

const REAL_STRICT_JSON: &str = include_str!("../profiles/strict.json");
const REAL_OCENTRA_PARENT_JSON: &str = include_str!("../profiles/ocentra-parent.json");
const REAL_PROJECT_CONFIG_JSON: &str = include_str!("../../../ocentra-enforcer.config.json");

// ---- 3-layer resolution cases -----------------------------------

#[test]
fn profile_only_case_strict() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = resolve_profile_only(&ConfigProfileName::new("strict".to_owned())?)?;
    assert_eq!(cfg.profile_name.as_str(), "strict");
    assert!(matches!(
        cfg.rust_scan_scope.fail_fast,
        RuleEnabled::Disabled
    ));
    Ok(())
}

#[test]
fn profile_plus_override_merge_case() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "schemaVersion": 2,
        "profileName": "ocentra-enforcer",
        "rustRoots": ["src", "crates", "custom-root"]
    })
    .to_string();
    let cfg = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("cfg.json".to_owned()),
    )?;
    assert_eq!(cfg.profile_name.as_str(), "ocentra-enforcer");
    assert_eq!(
        cfg.rust_scan_scope
            .rust_roots
            .iter()
            .map(|root| root.as_str())
            .collect::<Vec<_>>(),
        vec!["src", "crates", "custom-root"]
    );
    Ok(())
}

#[test]
fn unknown_profile_rejection_case() {
    let raw = json!({ "schemaVersion": 2, "profileName": "bogus" }).to_string();
    let result = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("cfg.json".to_owned()),
    );
    assert!(matches!(
        result,
        Err(ConfigLoadError::UnknownProfile { .. })
    ));
}

#[test]
fn missing_field_rejection_case() {
    let raw = json!({ "profileName": "strict" }).to_string();
    let result = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("cfg.json".to_owned()),
    );
    assert!(matches!(
        result,
        Err(ConfigLoadError::MissingRequiredField { .. })
    ));
}

// ---- [G5] per-field-group EffectiveConfig round-trip fixtures ----

#[test]
fn g5_shape_ownership_globs_populated_vs_empty() -> Result<(), Box<dyn std::error::Error>> {
    let strict = resolve_profile_only(&ConfigProfileName::new("strict".to_owned())?)?;
    assert!(strict.shape_ownership.raw_string_owner_globs.is_empty());
    assert!(strict
        .shape_ownership
        .domain_primitive_owner_globs
        .is_empty());

    let parent = resolve_profile_only(&ConfigProfileName::new("ocentra-parent".to_owned())?)?;
    assert_eq!(parent.shape_ownership.raw_string_owner_globs.len(), 10);
    assert_eq!(parent.shape_ownership.domain_primitive_owner_globs.len(), 6);
    assert_eq!(
        parent.shape_ownership.serialized_domain_owner_globs.len(),
        6
    );
    Ok(())
}

#[test]
fn g5_runtime_literal_policy_ocentra_parent_vs_strict() -> Result<(), Box<dyn std::error::Error>> {
    let strict = resolve_profile_only(&ConfigProfileName::new("strict".to_owned())?)?;
    assert!(matches!(
        strict
            .runtime_literal_policy
            .enforce_runtime_string_literals,
        RuleEnabled::Disabled
    ));

    let parent = resolve_profile_only(&ConfigProfileName::new("ocentra-parent".to_owned())?)?;
    assert!(matches!(
        parent
            .runtime_literal_policy
            .enforce_runtime_string_literals,
        RuleEnabled::Enabled
    ));
    // The 9 escaped-regex allow patterns must round-trip verbatim.
    let patterns = &parent
        .runtime_literal_policy
        .runtime_string_line_allow_patterns;
    assert_eq!(patterns.len(), 9);
    assert!(patterns.iter().any(|p| p.as_str() == r"env!\("));
    assert!(patterns.iter().any(|p| p.as_str() == r"#\[serde"));
    assert!(patterns
        .iter()
        .any(|p| p.as_str() == r#"^\s*(?:pub\s+)?const\s+[A-Z0-9_]+\s*:\s*&str\s*=\s*""#));
    Ok(())
}

#[test]
fn g5_cargo_dependency_policy_blocked_protocol_map_empty_vs_populated(
) -> Result<(), Box<dyn std::error::Error>> {
    let strict = resolve_profile_only(&ConfigProfileName::new("strict".to_owned())?)?;
    assert!(strict
        .cargo_dependency_policy
        .blocked_protocol_dependencies
        .is_empty());

    let parent = resolve_profile_only(&ConfigProfileName::new("ocentra-parent".to_owned())?)?;
    let blocked = &parent.cargo_dependency_policy.blocked_protocol_dependencies;
    assert_eq!(blocked.len(), 1);
    let forbidden = blocked
        .iter()
        .find(|(k, _)| k.as_str() == "ocentra-parent-agent-protocol")
        .map(|(_, v)| v)
        .ok_or("expected ocentra-parent-agent-protocol key")?;
    assert_eq!(forbidden.len(), 5);

    // Round-trip: serialize then re-resolve-equivalent map shape.
    let wire = serde_json::to_string(&WireEffectiveConfig::from(parent.clone()))?;
    let back: EffectiveConfig = decode_json::<WireEffectiveConfig>(
        &ConfigJson::from_owned(wire),
        &ConfigSource::from_owned("cargo policy round trip".to_owned()),
        "effective config wire must decode",
    )?
    .try_into()
    .map_err(std::io::Error::other)?;
    assert_eq!(
        back.cargo_dependency_policy.blocked_protocol_dependencies,
        *blocked
    );
    Ok(())
}

#[test]
fn g5_rust_roots_and_scan_scope_ocentra_parent_vs_strict() -> Result<(), Box<dyn std::error::Error>>
{
    let strict = resolve_profile_only(&ConfigProfileName::new("strict".to_owned())?)?;
    assert_eq!(
        strict
            .rust_scan_scope
            .rust_roots
            .iter()
            .map(|root| root.as_str())
            .collect::<Vec<_>>(),
        vec!["src", "crates", "tools"]
    );
    assert_eq!(strict.rust_scan_scope.cargo_test_threads, None);

    let parent = resolve_profile_only(&ConfigProfileName::new("ocentra-parent".to_owned())?)?;
    assert_eq!(
        parent
            .rust_scan_scope
            .rust_roots
            .iter()
            .map(|root| root.as_str())
            .collect::<Vec<_>>(),
        vec!["apps/parent-desktop/src-tauri", "crates", "tools"]
    );
    assert_eq!(parent.rust_scan_scope.cargo_test_threads, None);
    Ok(())
}

#[test]
fn g5_cargo_test_threads_numeric_value_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "schemaVersion": 2,
        "profileName": "strict",
        "cargoTestThreads": 4
    })
    .to_string();
    let cfg = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("cfg.json".to_owned()),
    )?;
    assert_eq!(
        cfg.rust_scan_scope.cargo_test_threads,
        std::num::NonZeroUsize::new(4)
    );
    let wire = serde_json::to_string(&WireEffectiveConfig::from(cfg.clone()))?;
    let back: EffectiveConfig = decode_json::<WireEffectiveConfig>(
        &ConfigJson::from_owned(wire),
        &ConfigSource::from_owned("cargo thread round trip".to_owned()),
        "effective config wire must decode",
    )?
    .try_into()
    .map_err(std::io::Error::other)?;
    assert_eq!(
        back.rust_scan_scope.cargo_test_threads,
        cfg.rust_scan_scope.cargo_test_threads
    );
    Ok(())
}

// ---- [G4] sourceShapePolicies base-shape fixture ------------------

#[test]
fn g4_source_shape_policies_round_trip_from_real_project_config(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = resolve(
        Some(&ConfigJson::from_owned(REAL_PROJECT_CONFIG_JSON.to_owned())),
        &ConfigSource::from_owned("ocentra-enforcer.config.json".to_owned()),
    )?;
    assert_eq!(cfg.source_shape_policies.len(), 3);

    let rust_entry = cfg
        .source_shape_policies
        .iter()
        .find(|p| p.kind == SourceShapeKind::Rust)
        .ok_or("expected a rust sourceShapePolicy entry")?;
    assert_eq!(
        rust_entry.max_types.map(std::num::NonZeroUsize::get),
        Some(24)
    );
    assert_eq!(
        rust_entry.max_classes, None,
        "rust entries do not set maxClasses"
    );

    let ts_entries: Vec<_> = cfg
        .source_shape_policies
        .iter()
        .filter(|p| p.kind == SourceShapeKind::Typescript)
        .collect();
    assert_eq!(ts_entries.len(), 2);
    Ok(())
}

// ---- ocentra-parent posture resolution fixture --------------------

#[test]
fn ocentra_parent_hardened_posture_resolves_without_loss() -> Result<(), Box<dyn std::error::Error>>
{
    let cfg = resolve_profile_only(&ConfigProfileName::new("ocentra-parent".to_owned())?)?;
    assert_eq!(
        cfg.cargo_dependency_policy.public_reexport_policy,
        PublicReexportPolicy::Forbid
    );
    assert!(matches!(
        cfg.runtime_literal_policy.enforce_runtime_string_literals,
        RuleEnabled::Enabled
    ));
    assert_eq!(cfg.shape_ownership.runtime_string_owner_globs.len(), 10);
    assert!(matches!(
        cfg.runtime_literal_policy
            .enforce_serialized_public_domain_primitives,
        RuleEnabled::Enabled
    ));
    assert_eq!(cfg.shape_ownership.serialized_domain_owner_globs.len(), 6);
    assert_eq!(
        cfg.cargo_dependency_policy
            .blocked_protocol_dependencies
            .len(),
        1
    );

    // Distinct from strict: same forbid posture, but empty owner globs
    // and literal-ban off.
    let strict = resolve_profile_only(&ConfigProfileName::new("strict".to_owned())?)?;
    assert_eq!(
        strict.cargo_dependency_policy.public_reexport_policy,
        PublicReexportPolicy::Forbid
    );
    assert!(matches!(
        strict
            .runtime_literal_policy
            .enforce_runtime_string_literals,
        RuleEnabled::Disabled
    ));
    assert!(strict.shape_ownership.runtime_string_owner_globs.is_empty());

    // Distinct from zero-config default: default has no hardening at
    // all engaged.
    let default_cfg = resolve(None, &ConfigSource::from_owned("<none>".to_owned()))?;
    assert_eq!(default_cfg.profile_name.as_str(), "default");
    assert!(matches!(
        default_cfg
            .runtime_literal_policy
            .enforce_runtime_string_literals,
        RuleEnabled::Disabled
    ));
    Ok(())
}

// ---- Cover-all completeness gate ----------------------------------

/// Recursively collect every leaf-bearing key path an object contains
/// (dotted notation; array indices collapse to `[]` since arrays hold
/// homogeneous typed elements here).
fn collect_key_paths(value: &Value, prefix: &str, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                out.push(path.clone());
                collect_key_paths(v, &path, out);
            }
        }
        Value::Array(items) => {
            if let Some(first) = items.first() {
                collect_key_paths(first, &format!("{prefix}[]"), out);
            }
        }
        _ => {}
    }
}

/// Known top-level keys that legacy config files may carry but which
/// are NOT part of `EffectiveConfig` because they are owned elsewhere
/// per the plan's disjoint-ownership design (rules/waivers/tools =
/// arc-04/arc-08 scope; sourceShapeOverrides = a08 dishonest-waiver
/// scope referencing the [G4] base shape but not owning it; failOn/
/// languages = arc-04/lang-* scope; ignoreFileGlobs entries under
/// `sourceShapeOverrides` handled separately).
const OUT_OF_SCOPE_TOP_LEVEL_KEYS: &[&str] = &[
    "rules",
    "waivers",
    "tools",
    "failOn",
    "languages",
    "sourceShapeOverrides",
];

#[test]
fn cover_all_every_strict_json_key_maps_to_a_typed_field() -> Result<(), Box<dyn std::error::Error>>
{
    let cfg = resolve_profile_only(&ConfigProfileName::new("strict".to_owned())?)?;
    let wire = serde_json::to_value(WireEffectiveConfig::from(cfg))?;
    let raw: Value = decode_json(
        &ConfigJson::from_owned(REAL_STRICT_JSON.to_owned()),
        &ConfigSource::from_owned("strict fixture".to_owned()),
        "strict fixture must decode",
    )?;
    assert_every_top_level_key_consumed(&raw, &wire)?;
    Ok(())
}

#[test]
fn cover_all_every_ocentra_parent_json_key_maps_to_a_typed_field(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = resolve_profile_only(&ConfigProfileName::new("ocentra-parent".to_owned())?)?;
    let wire = serde_json::to_value(WireEffectiveConfig::from(cfg))?;
    let raw: Value = decode_json(
        &ConfigJson::from_owned(REAL_OCENTRA_PARENT_JSON.to_owned()),
        &ConfigSource::from_owned("ocentra parent fixture".to_owned()),
        "ocentra parent fixture must decode",
    )?;
    assert_every_top_level_key_consumed(&raw, &wire)?;
    Ok(())
}

#[test]
fn cover_all_every_project_config_json_key_maps_to_a_typed_field(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = resolve(
        Some(&ConfigJson::from_owned(REAL_PROJECT_CONFIG_JSON.to_owned())),
        &ConfigSource::from_owned("ocentra-enforcer.config.json".to_owned()),
    )?;
    let wire = serde_json::to_value(WireEffectiveConfig::from(cfg))?;
    let raw: Value = decode_json(
        &ConfigJson::from_owned(REAL_PROJECT_CONFIG_JSON.to_owned()),
        &ConfigSource::from_owned("project fixture".to_owned()),
        "project fixture must decode",
    )?;
    assert_every_top_level_key_consumed(&raw, &wire)?;
    Ok(())
}

fn assert_every_top_level_key_consumed(
    raw: &Value,
    resolved_wire: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let raw_map = raw.as_object().ok_or("raw config must be a JSON object")?;
    let wire_map = resolved_wire
        .as_object()
        .ok_or("resolved EffectiveConfig must serialize to a JSON object")?;
    let mut unconsumed = Vec::new();
    for key in raw_map.keys() {
        if OUT_OF_SCOPE_TOP_LEVEL_KEYS.contains(&key.as_str()) {
            continue;
        }
        if !wire_map.contains_key(key) {
            unconsumed.push(key.clone());
        }
    }
    assert!(
            unconsumed.is_empty(),
            "every in-scope top-level config key must map to a typed EffectiveConfig field; unconsumed: {unconsumed:?}"
        );
    Ok(())
}

// Sanity: collect_key_paths is exercised so it stays live documentation
// of the nested-key shape even though the cover-all gate itself only
// needs the top-level check above.
#[test]
fn collect_key_paths_walks_nested_objects_and_arrays() {
    let value = json!({ "a": { "b": 1 }, "c": [ { "d": 2 } ] });
    let mut out = Vec::new();
    collect_key_paths(&value, "", &mut out);
    assert_eq!(out, vec!["a", "a.b", "c", "c[].d"]);
}

#[test]
fn platform_default_used_when_absent_from_all_layers() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = resolve_profile_only(&ConfigProfileName::new("ocentra-enforcer".to_owned())?)?;
    assert_eq!(cfg.supported_platforms, Platform::all());
    Ok(())
}

// ---- a07: rule ids parse-at-boundary into `RuleId`, not `String` --
//
// This is already satisfied by arc-03/f03's `policy::Policy::rule_toggles:
// BTreeMap<RuleId, RuleToggle>` â€” `serde` decodes the map's JSON string
// keys directly into `RuleId` (which itself validates via
// `RuleId::from_str`), so a config carrying a malformed rule id fails to
// load rather than smuggling a raw `String` through. Pinned here as an
// a07 proof fixture rather than reimplemented, per the reconciliation
// note in the a07 workpack.
#[test]
fn rule_ids_in_project_tie_policy_parse_at_boundary_into_ruleid_not_string(
) -> Result<(), Box<dyn std::error::Error>> {
    use enforcer_config::project_tie::parse_project_tie;
    let raw = json!({
        "policy": {
            "ruleToggles": {
                "RR-1.1": { "enabled": true }
            }
        }
    })
    .to_string();
    let resolved = parse_project_tie(
        &ConfigJson::from_owned(raw),
        &ConfigSource::from_owned("cfg.json".to_owned()),
    )?;
    let rule_id: enforcer_domain::ids::RuleId = "RR-1.1".parse()?;
    assert!(matches!(
        resolved.policy.rule_enabled(&rule_id),
        RuleEnabled::Enabled
    ));
    Ok(())
}

#[test]
fn malformed_rule_id_key_fails_closed_at_the_project_tie_boundary() {
    use enforcer_config::project_tie::parse_project_tie;
    let raw = json!({
        "policy": {
            "ruleToggles": {
                "not-a-rule-id": { "enabled": true }
            }
        }
    })
    .to_string();
    let outcome = parse_project_tie(
        &ConfigJson::from_owned(raw),
        &ConfigSource::from_owned("bad.json".to_owned()),
    );
    assert!(
        outcome.is_err(),
        "a malformed rule id key must fail to decode into RuleId, not pass through as String"
    );
}

#[cfg(test)]
mod env_integration_tests {
    //! a07 integration coverage for [`crate::load_project_config_with_env`]:
    //! the composition of [`crate::env::ConfigEnv`] (typed env-var decode)
    //! with the existing file-load pipeline. Unlike `env::tests` (which use
    //! a controlled [`crate::env::EnvLookup`] to avoid the real process
    //! environment), these tests exercise the real `std::env` var names to
    //! prove the end-to-end wiring; each test sets only the var(s) it needs
    //! and removes them before returning, and the var names
    //! (`ENFORCER_CONFIG_PATH`, `ENFORCER_PROFILE`) are unique to this
    //! crate's test suite.
    use enforcer_config::env::{ENFORCER_CONFIG_PATH_VAR, ENFORCER_PROFILE_VAR};
    use enforcer_config::load_project_config_with_env;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    fn process_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvVarGuard(&'static str);

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            std::env::remove_var(self.0);
        }
    }

    #[test]
    fn no_env_overrides_falls_back_to_default_path_behavior(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _lock = process_env_lock()
            .lock()
            .map_err(|_poison| std::io::Error::other("process environment lock poisoned"))?;
        std::env::remove_var(ENFORCER_CONFIG_PATH_VAR);
        std::env::remove_var(ENFORCER_PROFILE_VAR);
        let cfg = load_project_config_with_env(Path::new("<no such file>.json"))?;
        assert_eq!(cfg.profile_name.as_str(), "default");
        Ok(())
    }

    #[test]
    fn profile_env_override_wins_over_default_path_result() -> Result<(), Box<dyn std::error::Error>>
    {
        let _lock = process_env_lock()
            .lock()
            .map_err(|_poison| std::io::Error::other("process environment lock poisoned"))?;
        std::env::remove_var(ENFORCER_CONFIG_PATH_VAR);
        std::env::set_var(ENFORCER_PROFILE_VAR, "strict");
        let _guard = EnvVarGuard(ENFORCER_PROFILE_VAR);
        let cfg = load_project_config_with_env(Path::new("<no such file>.json"))?;
        assert_eq!(cfg.profile_name.as_str(), "strict");
        Ok(())
    }

    #[test]
    fn invalid_profile_env_override_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let _lock = process_env_lock()
            .lock()
            .map_err(|_poison| std::io::Error::other("process environment lock poisoned"))?;
        std::env::remove_var(ENFORCER_CONFIG_PATH_VAR);
        std::env::set_var(ENFORCER_PROFILE_VAR, "not-a-real-profile");
        let _guard = EnvVarGuard(ENFORCER_PROFILE_VAR);
        let outcome = load_project_config_with_env(Path::new("<no such file>.json"));
        assert!(
            outcome.is_err(),
            "an invalid ENFORCER_PROFILE value must fail closed, not silently fall back"
        );
        Ok(())
    }
}
