//! Three-layer config resolution: embedded/custom profile (defaults) ->
//! project config (local overrides, deep-merged) -> one typed
//! [`crate::model::EffectiveConfig`]. Zero project config resolves to the
//! `default` profile alone.

use enforcer_domain::config_types::{ConfigJson, ConfigProfileName, ConfigSource, EffectiveConfig};

use crate::error::ConfigResult;
use crate::serde::{resolve_json_layers, resolve_profile_json};

/// Resolve an `EffectiveConfig` from an optional raw project config JSON
/// string. `project_config_json = None` means "no project config exists at
/// all" -> the `default` profile alone is the effective config (zero-config
/// projects work out of the box). `source_path` is used only for error
/// messages.
pub fn resolve(
    project_config_json: Option<&ConfigJson>,
    source_path: &ConfigSource,
) -> ConfigResult<EffectiveConfig> {
    resolve_json_layers(project_config_json, source_path)
}

/// Resolve directly against a named profile with no project overrides Ã¢â‚¬â€
/// used by the "profile-only" fixture and by tooling that wants a pure
/// profile's `EffectiveConfig` (e.g. `enforcer doctor`).
pub fn resolve_profile_only(profile_name: &ConfigProfileName) -> ConfigResult<EffectiveConfig> {
    resolve_profile_json(profile_name)
}
