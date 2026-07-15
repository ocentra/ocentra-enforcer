//! Three-layer config resolution: embedded/custom profile (defaults) ->
//! project config (local overrides, deep-merged) -> one typed
//! [`crate::model::EffectiveConfig`]. Zero project config resolves to the
//! `default` profile alone.

use enforcer_domain::boundary::decode_error::DecodeError;
use serde_json::Value;

use crate::error::{ConfigLoadError, ConfigResult};
use crate::model::EffectiveConfig;
use crate::profiles::{embedded_profile_json, KNOWN_PROFILE_NAMES};

/// Deep-merge `overlay` onto `base` in place: objects merge key-by-key
/// (recursively); any non-object value (including arrays) in `overlay`
/// replaces the corresponding `base` value wholesale — arrays are not
/// concatenated, matching the legacy `.mjs` override semantics (a project
/// that overrides a list means it, in full).
fn deep_merge(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                match base_map.get_mut(key) {
                    Some(base_value) => deep_merge(base_value, overlay_value),
                    None => {
                        // CLONE-JUSTIFICATION: A newly merged map entry must own its key
                        // after this borrowed overlay value is released.
                        let owned_key = key.clone();
                        // CLONE-JUSTIFICATION: The merged base must retain an independent
                        // value after the borrowed overlay is released.
                        let owned_value = overlay_value.clone();
                        base_map.insert(owned_key, owned_value);
                    }
                }
            }
        }
        (base_slot, overlay_value) => {
            // CLONE-JUSTIFICATION: Replacing the base requires an owned value that remains
            // valid after the borrowed overlay is released.
            *base_slot = overlay_value.clone();
        }
    }
}

/// Validate the mechanical `CFG-1.10`/`CFG-1.11` invariants on a raw project
/// config JSON value before it participates in merge: `schemaVersion` and
/// `profileName` must both be present, and `profileName` must name a known
/// profile.
fn validate_project_config_shape(source_path: &str, value: &Value) -> ConfigResult<String> {
    let object = value.as_object().ok_or_else(|| {
        ConfigLoadError::Parse(DecodeError::new(
            source_path,
            "project config must be a JSON object",
        ))
    })?;

    if !object.contains_key("schemaVersion") {
        return Err(ConfigLoadError::MissingRequiredField {
            path: source_path.to_owned(),
            field: "schemaVersion",
        });
    }

    let profile_name = object
        .get("profileName")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let profile_name = match profile_name {
        Some(name) => name,
        None => {
            return Err(ConfigLoadError::MissingRequiredField {
                path: source_path.to_owned(),
                field: "profileName",
            })
        }
    };

    if !KNOWN_PROFILE_NAMES.contains(&profile_name.as_str()) {
        return Err(ConfigLoadError::UnknownProfile {
            path: source_path.to_owned(),
            profile_name,
        });
    }

    Ok(profile_name)
}

/// Resolve an `EffectiveConfig` from an optional raw project config JSON
/// string. `project_config_json = None` means "no project config exists at
/// all" -> the `default` profile alone is the effective config (zero-config
/// projects work out of the box). `source_path` is used only for error
/// messages.
pub fn resolve(
    project_config_json: Option<&str>,
    source_path: &str,
) -> ConfigResult<EffectiveConfig> {
    let (profile_name, project_value) = match project_config_json {
        None => ("default".to_owned(), None),
        Some(raw) => {
            let value: Value = serde_json::from_str(raw).map_err(|e| {
                ConfigLoadError::Parse(DecodeError::new(source_path, format!("invalid JSON: {e}")))
            })?;
            let profile_name = validate_project_config_shape(source_path, &value)?;
            (profile_name, Some(value))
        }
    };

    let profile_json = embedded_profile_json(&profile_name)?;
    let mut merged: Value = serde_json::from_str(profile_json).map_err(|e| {
        ConfigLoadError::Parse(DecodeError::new(
            "<embedded profile>",
            format!("embedded profile `{profile_name}` failed to parse as JSON: {e}"),
        ))
    })?;

    if let Some(overlay) = &project_value {
        deep_merge(&mut merged, overlay);
    }

    serde_json::from_value(merged).map_err(|e| {
        ConfigLoadError::Parse(DecodeError::new(
            source_path,
            format!("resolved config did not decode into EffectiveConfig: {e}"),
        ))
    })
}

/// Resolve directly against a named profile with no project overrides —
/// used by the "profile-only" fixture and by tooling that wants a pure
/// profile's `EffectiveConfig` (e.g. `enforcer doctor`).
pub fn resolve_profile_only(profile_name: &str) -> ConfigResult<EffectiveConfig> {
    let profile_json = embedded_profile_json(profile_name)?;
    let value: Value = serde_json::from_str(profile_json).map_err(|e| {
        ConfigLoadError::Parse(DecodeError::new(
            "<embedded profile>",
            format!("embedded profile `{profile_name}` failed to parse as JSON: {e}"),
        ))
    })?;
    serde_json::from_value(value).map_err(|e| {
        ConfigLoadError::Parse(DecodeError::new(
            "<embedded profile>",
            format!("embedded profile `{profile_name}` did not decode into EffectiveConfig: {e}"),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::{deep_merge, resolve, resolve_profile_only};
    use crate::error::ConfigLoadError;
    use crate::model::{Platform, PublicReexportPolicy};
    use serde_json::json;

    #[test]
    fn zero_config_falls_back_to_default_profile_alone() -> Result<(), Box<dyn std::error::Error>> {
        let cfg = resolve(None, "<none>")?;
        assert_eq!(cfg.profile_name, "default");
        assert_eq!(cfg.supported_platforms, Platform::all());
        Ok(())
    }

    #[test]
    fn valid_project_config_merges_overrides_over_profile_defaults(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = json!({
            "schemaVersion": 2,
            "profileName": "strict",
            "failFast": true
        })
        .to_string();
        let cfg = resolve(Some(&raw), "ocentra-enforcer.config.json")?;
        assert_eq!(cfg.profile_name, "strict");
        assert!(
            cfg.rust_scan_scope.fail_fast,
            "override should win over profile default (false)"
        );
        assert_eq!(
            cfg.cargo_dependency_policy.public_reexport_policy,
            PublicReexportPolicy::Forbid
        );
        Ok(())
    }

    #[test]
    fn missing_schema_version_fails_typed() -> Result<(), Box<dyn std::error::Error>> {
        let raw = json!({ "profileName": "strict" }).to_string();
        let outcome = resolve(Some(&raw), "bad.json");
        let Err(err) = outcome else {
            return Err("expected Err for missing schemaVersion".into());
        };
        assert!(matches!(
            err,
            ConfigLoadError::MissingRequiredField {
                field: "schemaVersion",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn missing_profile_name_fails_typed() -> Result<(), Box<dyn std::error::Error>> {
        let raw = json!({ "schemaVersion": 2 }).to_string();
        let outcome = resolve(Some(&raw), "bad.json");
        let Err(err) = outcome else {
            return Err("expected Err for missing profileName".into());
        };
        assert!(matches!(
            err,
            ConfigLoadError::MissingRequiredField {
                field: "profileName",
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn unknown_profile_name_fails_typed_naming_it() -> Result<(), Box<dyn std::error::Error>> {
        let raw = json!({ "schemaVersion": 2, "profileName": "totally-made-up" }).to_string();
        let outcome = resolve(Some(&raw), "bad.json");
        let Err(err) = outcome else {
            return Err("expected Err for unknown profileName".into());
        };
        match err {
            ConfigLoadError::UnknownProfile { profile_name, .. } => {
                assert_eq!(profile_name, "totally-made-up");
                Ok(())
            }
            other => Err(format!("expected UnknownProfile, got {other:?}").into()),
        }
    }

    #[test]
    fn supported_platforms_present_vs_absent_defaults_to_all_three(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let absent = resolve_profile_only("strict")?;
        assert_eq!(absent.supported_platforms, Platform::all());

        let raw = json!({
            "schemaVersion": 2,
            "profileName": "strict",
            "supportedPlatforms": ["windows"]
        })
        .to_string();
        let present = resolve(Some(&raw), "cfg.json")?;
        assert_eq!(present.supported_platforms, vec![Platform::Windows]);
        Ok(())
    }

    #[test]
    fn deep_merge_replaces_arrays_wholesale_not_concatenated() {
        let mut base = json!({ "list": [1, 2, 3], "nested": { "a": 1, "b": 2 } });
        let overlay = json!({ "list": [9], "nested": { "b": 20 } });
        deep_merge(&mut base, &overlay);
        assert_eq!(base["list"], json!([9]));
        assert_eq!(base["nested"]["a"], json!(1));
        assert_eq!(base["nested"]["b"], json!(20));
    }
}
