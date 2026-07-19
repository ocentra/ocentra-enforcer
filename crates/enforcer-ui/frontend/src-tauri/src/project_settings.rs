//! Typed Tauri commands for project settings and scan-scope persistence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use enforcer_domain::paths::RepoRoot;
use serde::{Deserialize, Serialize};

// ROUNDTRIP-TEST: `scan_scope_settings_request_round_trips_through_json`
// proves the persisted desktop request contract preserves every field.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanScopeSettingsPayload {
    pub(crate) source_path: String,
    pub(crate) exists: bool,
    pub(crate) profile_name: String,
    pub(crate) ignore_dirs: Vec<String>,
    pub(crate) ignore_file_globs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanScopeSettingsRequest {
    pub(crate) profile_name: String,
    pub(crate) ignore_dirs: Vec<String>,
    pub(crate) ignore_file_globs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScanScopeSettingsUpdate {
    profile_name: String,
    ignore_dirs: Vec<String>,
    ignore_file_globs: Vec<String>,
}

impl From<ScanScopeSettingsRequest> for ScanScopeSettingsUpdate {
    fn from(request: ScanScopeSettingsRequest) -> Self {
        Self {
            profile_name: request.profile_name,
            ignore_dirs: request.ignore_dirs,
            ignore_file_globs: request.ignore_file_globs,
        }
    }
}

#[tauri::command]
pub fn load_project_settings(root: String) -> Result<serde_json::Value, String> {
    let root = project_root(root)?;
    let config_path = project_path(&root).join("enforce.config.json");
    let payload =
        enforcer_ui::settings::read::load_settings_view(&config_path).map_err(|error| {
            format!(
                "cannot load project settings from {}: {error}",
                config_path.display()
            )
        })?;
    serde_json::to_value(payload)
        .map_err(|error| format!("cannot encode project settings view: {error}"))
}

#[tauri::command]
pub fn load_scan_scope_settings(root: String) -> Result<ScanScopeSettingsPayload, String> {
    read_scan_scope_settings(&project_root(root)?)
}

#[tauri::command]
pub fn write_scan_scope_settings(
    root: String,
    request: ScanScopeSettingsRequest,
) -> Result<ScanScopeSettingsPayload, String> {
    let root = project_root(root)?;
    let request = ScanScopeSettingsUpdate::from(request);
    let config_path = scan_scope_config_path(&root);
    let mut config = if config_path.is_file() {
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(&config_path)
                .map_err(|error| format!("cannot read scan policy config: {error}"))?,
        )
        .map_err(|error| format!("cannot decode scan policy config: {error}"))?
    } else {
        serde_json::json!({ "schemaVersion": 2, "profileName": request.profile_name })
    };
    let object = config
        .as_object_mut()
        .ok_or_else(|| "scan policy config must be a JSON object".to_owned())?;
    if !object.contains_key("schemaVersion") {
        return Err("existing scan policy config is missing schemaVersion".to_owned());
    }
    object.insert(
        "profileName".to_owned(),
        serde_json::Value::String(request.profile_name.trim().to_owned()),
    );
    object.insert(
        "ignoreDirs".to_owned(),
        serde_json::to_value(normalize_scan_patterns(request.ignore_dirs, "directory")?)
            .map_err(|error| format!("cannot encode ignore directories: {error}"))?,
    );
    object.insert(
        "ignoreFileGlobs".to_owned(),
        serde_json::to_value(normalize_scan_patterns(
            request.ignore_file_globs,
            "file glob",
        )?)
        .map_err(|error| format!("cannot encode ignore file globs: {error}"))?,
    );
    let encoded = serde_json::to_string_pretty(&config)
        .map_err(|error| format!("cannot encode scan policy config: {error}"))?
        + "\n";
    let pending_path = config_path.with_extension("json.pending");
    std::fs::write(&pending_path, &encoded)
        .map_err(|error| format!("cannot stage scan policy config: {error}"))?;
    let validation = enforcer_config::load_project_config(&pending_path)
        .map_err(|error| format!("scan policy config rejected by typed validation: {error}"));
    let _ = std::fs::remove_file(&pending_path);
    validation?;
    std::fs::write(&config_path, encoded)
        .map_err(|error| format!("cannot persist scan policy config: {error}"))?;
    read_scan_scope_settings(&root)
}

#[tauri::command]
pub fn write_rule_override(
    root: String,
    request: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let root = project_root(root)?;
    let config_path = project_path(&root).join("enforce.config.json");
    let request = enforcer_ui::settings::write::ToggleRuleInput::parse(&request)
        .map_err(|error| format!("invalid policy change: {error}"))?;
    let resolved = enforcer_ui::settings::write::toggle_rule(&config_path, &request)
        .map_err(|error| format!("cannot persist policy change: {error}"))?;
    serde_json::to_value(enforcer_ui::settings::read::render_settings_view(
        &config_path.display().to_string(),
        &resolved,
    ))
    .map_err(|error| format!("cannot encode updated project settings: {error}"))
}

fn project_root(root: String) -> Result<RepoRoot, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let canonical = root_path
        .canonicalize()
        .map_err(|error| format!("cannot resolve project root: {error}"))?;
    RepoRoot::try_from(canonical.as_path())
        .map_err(|error| format!("invalid project root: {error}"))
}

fn project_path(root: &RepoRoot) -> &Path {
    Path::new(root.as_str())
}

fn read_scan_scope_settings(root: &RepoRoot) -> Result<ScanScopeSettingsPayload, String> {
    let config_path = scan_scope_config_path(root);
    let effective = enforcer_config::load_project_config(&config_path)
        .map_err(|error| format!("cannot load typed scan policy config: {error}"))?;
    Ok(ScanScopeSettingsPayload {
        source_path: config_path.display().to_string(),
        exists: config_path.is_file(),
        profile_name: effective.profile_name.as_str().to_owned(),
        ignore_dirs: effective
            .ignore_dirs
            .into_iter()
            .map(|directory| directory.as_str().to_owned())
            .collect(),
        ignore_file_globs: effective
            .ignore_file_globs
            .into_iter()
            .map(|glob| glob.as_str().to_owned())
            .collect(),
    })
}

fn scan_scope_config_path(root: &RepoRoot) -> PathBuf {
    project_path(root).join("ocentra-enforcer.config.json")
}

fn normalize_scan_patterns(values: Vec<String>, kind: &str) -> Result<Vec<String>, String> {
    let mut patterns = BTreeSet::new();
    for value in values {
        let pattern = value.trim().replace('\\', "/");
        if pattern.is_empty() {
            continue;
        }
        if pattern.starts_with('/')
            || pattern.contains(":/")
            || pattern.split('/').any(|segment| segment == "..")
        {
            return Err(format!(
                "scan ignore {kind} must be project-relative: {pattern}"
            ));
        }
        patterns.insert(pattern);
    }
    Ok(patterns.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::ScanScopeSettingsRequest;

    #[test]
    fn scan_scope_settings_request_round_trips_through_json(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let round_trip_request = ScanScopeSettingsRequest {
            profile_name: "strict".to_owned(),
            ignore_dirs: vec!["target".to_owned()],
            ignore_file_globs: vec!["**/*.generated.rs".to_owned()],
        };
        let wire = serde_json::to_string(&round_trip_request)?;
        let restored: ScanScopeSettingsRequest = serde_json::from_str(&wire)?;
        assert_eq!(restored, round_trip_request);
        Ok(())
    }
}
