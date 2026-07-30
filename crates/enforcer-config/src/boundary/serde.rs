//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Serde-only ingress and egress shapes for configuration files.
//!
//! Canonical configuration values live in `enforcer-domain`. This module owns
//! every JSON spelling, default, and flatten rule, then converts once at the
//! configuration boundary.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::Path;

use enforcer_domain::config_types::{
    ArchitecturePolicyCheck, CargoDependencyPolicy, CfgTestSkipping, ConfigField, ConfigJson,
    ConfigProfileName, ConfigSource, CrateName, EffectiveConfig, EnforcerScope, Glob,
    HarnessArtifactByteLimit, HarnessConfig, HarnessRetentionDays, HarnessRunLimit,
    InlineTestPolicy, NativeMode, NativeTie, NativeTool, Platform, PolicyOwner, PolicyReason,
    PrivateRustTestModuleAllowlistEntry, PublicReexportPolicy, RegexPattern, RuleEnabled,
    RuntimeLiteralPolicy, RustScanScope, ShapeOwnershipGlobs, SourceShapeKind, SourceShapeOverride,
    SourceShapePolicy,
};
use enforcer_domain::{
    ids::RuleId, paths::RelPath, scan_types::IgnoreDirectorySegment, severity::Severity,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ConfigLoadError, ConfigResult};
use crate::profiles::KNOWN_PROFILE_NAMES;
use crate::project_tie::ProjectConfig;
use enforcer_domain::boundary::decode_error::DecodeError;

const STRICT_JSON: &str = include_str!("../../profiles/strict.json");
const DEFAULT_JSON: &str = include_str!("../../profiles/default.json");
const OCENTRA_ENFORCER_JSON: &str = include_str!("../../profiles/ocentra-enforcer.json");
const OCENTRA_PARENT_JSON: &str = include_str!("../../profiles/ocentra-parent.json");

const fn decode_rule_enabled(value: bool) -> RuleEnabled {
    if value {
        RuleEnabled::Enabled
    } else {
        RuleEnabled::Disabled
    }
}

const fn encode_rule_enabled(value: RuleEnabled) -> bool {
    matches!(value, RuleEnabled::Enabled)
}

const fn decode_cfg_test_skipping(value: bool) -> CfgTestSkipping {
    if value {
        CfgTestSkipping::Enabled
    } else {
        CfgTestSkipping::Disabled
    }
}

const fn encode_cfg_test_skipping(value: CfgTestSkipping) -> bool {
    matches!(value, CfgTestSkipping::Enabled)
}

/// Read a named embedded profile as boundary JSON.
pub fn embedded_profile_json(profile_name: &ConfigProfileName) -> ConfigResult<ConfigJson> {
    let source = ConfigSource::from_owned("<embedded profile lookup>".to_owned());
    let json = match profile_name.as_str() {
        "strict" => STRICT_JSON,
        "default" => DEFAULT_JSON,
        "ocentra-enforcer" => OCENTRA_ENFORCER_JSON,
        "ocentra-parent" => OCENTRA_PARENT_JSON,
        _ => {
            return Err(ConfigLoadError::UnknownProfile {
                path: source,
                profile_name: profile_name.clone(),
            })
        }
    };
    Ok(ConfigJson::from_owned(json.to_owned()))
}

/// Return every embedded profile as a canonical profile-name value.
///
/// # Errors
/// Returns a decode error if an embedded profile wire constant violates the
/// canonical profile-name invariant.
pub fn embedded_profile_names() -> Result<[ConfigProfileName; 4], DecodeError> {
    Ok([
        ConfigProfileName::new(String::from("strict"))?,
        ConfigProfileName::new(String::from("default"))?,
        ConfigProfileName::new(String::from("ocentra-enforcer"))?,
        ConfigProfileName::new(String::from("ocentra-parent"))?,
    ])
}

/// Read a configuration file into its boundary representation once.
pub(crate) fn read_config_json(path: &Path) -> ConfigResult<Option<(ConfigJson, ConfigSource)>> {
    if !path.exists() {
        return Ok(None);
    }
    let source = ConfigSource::from_owned(path.display().to_string());
    let raw = std::fs::read_to_string(path).map_err(|error| ConfigLoadError::Io {
        path: source.clone(),
        reason: enforcer_domain::config_types::ConfigErrorReason::from_owned(error.to_string()),
    })?;
    Ok(Some((ConfigJson::from_owned(raw), source)))
}

/// Construct the source identity for a concrete configuration-file boundary.
#[must_use]
pub fn config_source_for_path(path: &Path) -> ConfigSource {
    ConfigSource::from_owned(path.display().to_string())
}

/// Source marker for an absent project configuration.
pub(crate) fn absent_project_config_source() -> ConfigSource {
    ConfigSource::from_owned("<no project config>".to_owned())
}

/// Source marker for an absent `.enforce/config` file.
pub(crate) fn absent_project_tie_source() -> ConfigSource {
    ConfigSource::from_owned("<no .enforce/config>".to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WirePlatform {
    Windows,
    Macos,
    Linux,
}

impl From<WirePlatform> for Platform {
    fn from(value: WirePlatform) -> Self {
        match value {
            WirePlatform::Windows => Self::Windows,
            WirePlatform::Macos => Self::Macos,
            WirePlatform::Linux => Self::Linux,
        }
    }
}

impl From<Platform> for WirePlatform {
    fn from(value: Platform) -> Self {
        match value {
            Platform::Windows => Self::Windows,
            Platform::Macos => Self::Macos,
            Platform::Linux => Self::Linux,
        }
    }
}

fn default_supported_platforms() -> Vec<WirePlatform> {
    Platform::all().into_iter().map(Into::into).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WireCrateName(pub String);

impl TryFrom<WireCrateName> for CrateName {
    type Error = DecodeError;

    fn try_from(value: WireCrateName) -> Result<Self, Self::Error> {
        Self::try_from(value.0)
    }
}
impl From<CrateName> for WireCrateName {
    fn from(value: CrateName) -> Self {
        Self(value.as_str().to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WireGlob(pub String);

impl TryFrom<WireGlob> for Glob {
    type Error = DecodeError;

    fn try_from(value: WireGlob) -> Result<Self, Self::Error> {
        Self::new(value.0)
    }
}
impl From<Glob> for WireGlob {
    fn from(value: Glob) -> Self {
        Self(value.as_str().to_owned())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WirePublicReexportPolicy {
    Forbid,
    Allow,
}
impl From<WirePublicReexportPolicy> for PublicReexportPolicy {
    fn from(value: WirePublicReexportPolicy) -> Self {
        match value {
            WirePublicReexportPolicy::Forbid => Self::Forbid,
            WirePublicReexportPolicy::Allow => Self::Allow,
        }
    }
}
impl From<PublicReexportPolicy> for WirePublicReexportPolicy {
    fn from(value: PublicReexportPolicy) -> Self {
        match value {
            PublicReexportPolicy::Forbid => Self::Forbid,
            PublicReexportPolicy::Allow => Self::Allow,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WireInlineTestPolicy {
    #[default]
    Forbid,
    Warn,
    Allow,
}
impl From<WireInlineTestPolicy> for InlineTestPolicy {
    fn from(value: WireInlineTestPolicy) -> Self {
        match value {
            WireInlineTestPolicy::Forbid => Self::Forbid,
            WireInlineTestPolicy::Warn => Self::Warn,
            WireInlineTestPolicy::Allow => Self::Allow,
        }
    }
}
impl From<InlineTestPolicy> for WireInlineTestPolicy {
    fn from(value: InlineTestPolicy) -> Self {
        match value {
            InlineTestPolicy::Forbid => Self::Forbid,
            InlineTestPolicy::Warn => Self::Warn,
            InlineTestPolicy::Allow => Self::Allow,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireHarnessConfig {
    pub store: String,
    pub storage_dir: String,
    pub max_artifact_bytes: u64,
    pub max_runs: Option<u64>,
    pub max_runs_per_tool: Option<u64>,
    pub max_failed_runs: Option<u64>,
    pub prune_after_days: Option<u64>,
}
impl Default for WireHarnessConfig {
    fn default() -> Self {
        let value = HarnessConfig::default();
        Self {
            store: value.store.as_str().to_owned(),
            storage_dir: value.storage_dir.as_str().to_owned(),
            max_artifact_bytes: value.max_artifact_bytes.get(),
            max_runs: value.max_runs.map(HarnessRunLimit::get),
            max_runs_per_tool: value.max_runs_per_tool.map(HarnessRunLimit::get),
            max_failed_runs: value.max_failed_runs.map(HarnessRunLimit::get),
            prune_after_days: value.prune_after_days.map(HarnessRetentionDays::get),
        }
    }
}
impl From<WireHarnessConfig> for HarnessConfig {
    fn from(value: WireHarnessConfig) -> Self {
        Self {
            store: ConfigSource::from_owned(value.store),
            storage_dir: ConfigSource::from_owned(value.storage_dir),
            max_artifact_bytes: HarnessArtifactByteLimit::from_value(value.max_artifact_bytes),
            max_runs: value.max_runs.map(HarnessRunLimit::from_value),
            max_runs_per_tool: value.max_runs_per_tool.map(HarnessRunLimit::from_value),
            max_failed_runs: value.max_failed_runs.map(HarnessRunLimit::from_value),
            prune_after_days: value.prune_after_days.map(HarnessRetentionDays::from_value),
        }
    }
}
impl From<HarnessConfig> for WireHarnessConfig {
    fn from(value: HarnessConfig) -> Self {
        Self {
            store: value.store.as_str().to_owned(),
            storage_dir: value.storage_dir.as_str().to_owned(),
            max_artifact_bytes: value.max_artifact_bytes.get(),
            max_runs: value.max_runs.map(HarnessRunLimit::get),
            max_runs_per_tool: value.max_runs_per_tool.map(HarnessRunLimit::get),
            max_failed_runs: value.max_failed_runs.map(HarnessRunLimit::get),
            prune_after_days: value.prune_after_days.map(HarnessRetentionDays::get),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireShapeOwnershipGlobs {
    #[serde(default)]
    pub raw_type_boundary_globs: Vec<WireGlob>,
    #[serde(default)]
    pub facade_file_globs: Vec<WireGlob>,
    #[serde(default)]
    pub raw_string_owner_globs: Vec<WireGlob>,
    #[serde(default)]
    pub domain_primitive_owner_globs: Vec<WireGlob>,
    #[serde(default)]
    pub serialized_domain_owner_globs: Vec<WireGlob>,
    #[serde(default)]
    pub runtime_string_owner_globs: Vec<WireGlob>,
}
impl TryFrom<WireShapeOwnershipGlobs> for ShapeOwnershipGlobs {
    type Error = DecodeError;

    fn try_from(value: WireShapeOwnershipGlobs) -> Result<Self, Self::Error> {
        Ok(Self {
            raw_type_boundary_globs: value
                .raw_type_boundary_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            facade_file_globs: value
                .facade_file_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            raw_string_owner_globs: value
                .raw_string_owner_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            domain_primitive_owner_globs: value
                .domain_primitive_owner_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            serialized_domain_owner_globs: value
                .serialized_domain_owner_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            runtime_string_owner_globs: value
                .runtime_string_owner_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}
impl From<ShapeOwnershipGlobs> for WireShapeOwnershipGlobs {
    fn from(value: ShapeOwnershipGlobs) -> Self {
        Self {
            raw_type_boundary_globs: value
                .raw_type_boundary_globs
                .into_iter()
                .map(Into::into)
                .collect(),
            facade_file_globs: value
                .facade_file_globs
                .into_iter()
                .map(Into::into)
                .collect(),
            raw_string_owner_globs: value
                .raw_string_owner_globs
                .into_iter()
                .map(Into::into)
                .collect(),
            domain_primitive_owner_globs: value
                .domain_primitive_owner_globs
                .into_iter()
                .map(Into::into)
                .collect(),
            serialized_domain_owner_globs: value
                .serialized_domain_owner_globs
                .into_iter()
                .map(Into::into)
                .collect(),
            runtime_string_owner_globs: value
                .runtime_string_owner_globs
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireRuntimeLiteralPolicy {
    #[serde(default)]
    pub enforce_runtime_string_literals: bool,
    #[serde(default)]
    pub runtime_string_line_allow_patterns: Vec<String>,
    #[serde(default)]
    pub enforce_serialized_public_domain_primitives: bool,
    #[serde(default)]
    pub enforce_workspace_files: bool,
}
impl TryFrom<WireRuntimeLiteralPolicy> for RuntimeLiteralPolicy {
    type Error = DecodeError;

    fn try_from(value: WireRuntimeLiteralPolicy) -> Result<Self, Self::Error> {
        Ok(Self {
            enforce_runtime_string_literals: decode_rule_enabled(
                value.enforce_runtime_string_literals,
            ),
            runtime_string_line_allow_patterns: value
                .runtime_string_line_allow_patterns
                .into_iter()
                .map(RegexPattern::new)
                .collect::<Result<_, _>>()?,
            enforce_serialized_public_domain_primitives: decode_rule_enabled(
                value.enforce_serialized_public_domain_primitives,
            ),
            enforce_workspace_files: decode_rule_enabled(value.enforce_workspace_files),
        })
    }
}
impl From<RuntimeLiteralPolicy> for WireRuntimeLiteralPolicy {
    fn from(value: RuntimeLiteralPolicy) -> Self {
        Self {
            enforce_runtime_string_literals: encode_rule_enabled(
                value.enforce_runtime_string_literals,
            ),
            runtime_string_line_allow_patterns: value
                .runtime_string_line_allow_patterns
                .into_iter()
                .map(|pattern| pattern.as_str().to_owned())
                .collect(),
            enforce_serialized_public_domain_primitives: encode_rule_enabled(
                value.enforce_serialized_public_domain_primitives,
            ),
            enforce_workspace_files: encode_rule_enabled(value.enforce_workspace_files),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireCargoDependencyPolicy {
    #[serde(default)]
    pub blocked_protocol_dependencies: BTreeMap<WireCrateName, Vec<WireCrateName>>,
    #[serde(default)]
    pub runtime_crates: Vec<WireCrateName>,
    #[serde(default)]
    pub test_only_crates: Vec<WireCrateName>,
    #[serde(default)]
    pub allowed_git_dependencies: Vec<WireCrateName>,
    #[serde(default)]
    pub allowed_build_rs_paths: Vec<String>,
    #[serde(default)]
    pub allow_git_dependencies: bool,
    #[serde(default)]
    pub allow_path_dependencies: bool,
    #[serde(default)]
    pub allow_build_rs: bool,
    #[serde(default)]
    pub allow_unsafe_code: bool,
    pub public_reexport_policy: WirePublicReexportPolicy,
    #[serde(default)]
    pub require_cargo_deny: bool,
    #[serde(default)]
    pub require_cargo_audit: bool,
}
fn crate_names(values: Vec<WireCrateName>) -> Result<Vec<CrateName>, DecodeError> {
    values.into_iter().map(CrateName::try_from).collect()
}

fn crate_name_map(
    values: BTreeMap<WireCrateName, Vec<WireCrateName>>,
) -> Result<BTreeMap<CrateName, Vec<CrateName>>, DecodeError> {
    values
        .into_iter()
        .map(|(key, value)| Ok((CrateName::try_from(key)?, crate_names(value)?)))
        .collect()
}

impl TryFrom<WireCargoDependencyPolicy> for CargoDependencyPolicy {
    type Error = DecodeError;

    fn try_from(value: WireCargoDependencyPolicy) -> Result<Self, Self::Error> {
        Ok(Self {
            blocked_protocol_dependencies: crate_name_map(value.blocked_protocol_dependencies)?,
            runtime_crates: crate_names(value.runtime_crates)?,
            test_only_crates: crate_names(value.test_only_crates)?,
            allowed_git_dependencies: crate_names(value.allowed_git_dependencies)?,
            allowed_build_rs_paths: value
                .allowed_build_rs_paths
                .into_iter()
                .map(RelPath::try_from)
                .collect::<Result<_, _>>()?,
            allow_git_dependencies: decode_rule_enabled(value.allow_git_dependencies),
            allow_path_dependencies: decode_rule_enabled(value.allow_path_dependencies),
            allow_build_rs: decode_rule_enabled(value.allow_build_rs),
            allow_unsafe_code: decode_rule_enabled(value.allow_unsafe_code),
            public_reexport_policy: value.public_reexport_policy.into(),
            require_cargo_deny: decode_rule_enabled(value.require_cargo_deny),
            require_cargo_audit: decode_rule_enabled(value.require_cargo_audit),
        })
    }
}
impl From<CargoDependencyPolicy> for WireCargoDependencyPolicy {
    fn from(value: CargoDependencyPolicy) -> Self {
        Self {
            blocked_protocol_dependencies: value
                .blocked_protocol_dependencies
                .into_iter()
                .map(|(key, values)| (key.into(), values.into_iter().map(Into::into).collect()))
                .collect(),
            runtime_crates: value.runtime_crates.into_iter().map(Into::into).collect(),
            test_only_crates: value.test_only_crates.into_iter().map(Into::into).collect(),
            allowed_git_dependencies: value
                .allowed_git_dependencies
                .into_iter()
                .map(Into::into)
                .collect(),
            allowed_build_rs_paths: value
                .allowed_build_rs_paths
                .into_iter()
                .map(Into::into)
                .collect(),
            allow_git_dependencies: encode_rule_enabled(value.allow_git_dependencies),
            allow_path_dependencies: encode_rule_enabled(value.allow_path_dependencies),
            allow_build_rs: encode_rule_enabled(value.allow_build_rs),
            allow_unsafe_code: encode_rule_enabled(value.allow_unsafe_code),
            public_reexport_policy: value.public_reexport_policy.into(),
            require_cargo_deny: encode_rule_enabled(value.require_cargo_deny),
            require_cargo_audit: encode_rule_enabled(value.require_cargo_audit),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireRustScanScope {
    #[serde(default)]
    pub rust_roots: Vec<String>,
    #[serde(default)]
    pub crate_root_globs: Vec<WireGlob>,
    #[serde(default)]
    pub test_file_globs: Vec<WireGlob>,
    #[serde(default)]
    pub inline_test_policy: WireInlineTestPolicy,
    #[serde(default)]
    pub cargo_on_file_scope: bool,
    #[serde(default)]
    pub cargo_on_diff_scope: bool,
    #[serde(default)]
    pub cargo_test_threads: Option<NonZeroUsize>,
    #[serde(default)]
    pub run_cargo_doc: bool,
    #[serde(default)]
    pub fail_fast: bool,
}
impl TryFrom<WireRustScanScope> for RustScanScope {
    type Error = DecodeError;

    fn try_from(value: WireRustScanScope) -> Result<Self, Self::Error> {
        Ok(Self {
            rust_roots: value
                .rust_roots
                .into_iter()
                .map(RelPath::try_from)
                .collect::<Result<_, _>>()?,
            crate_root_globs: value
                .crate_root_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            test_file_globs: value
                .test_file_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            inline_test_policy: value.inline_test_policy.into(),
            cargo_on_file_scope: decode_rule_enabled(value.cargo_on_file_scope),
            cargo_on_diff_scope: decode_rule_enabled(value.cargo_on_diff_scope),
            cargo_test_threads: value.cargo_test_threads,
            run_cargo_doc: decode_rule_enabled(value.run_cargo_doc),
            fail_fast: decode_rule_enabled(value.fail_fast),
        })
    }
}
impl From<RustScanScope> for WireRustScanScope {
    fn from(value: RustScanScope) -> Self {
        Self {
            rust_roots: value.rust_roots.into_iter().map(String::from).collect(),
            crate_root_globs: value.crate_root_globs.into_iter().map(Into::into).collect(),
            test_file_globs: value.test_file_globs.into_iter().map(Into::into).collect(),
            inline_test_policy: value.inline_test_policy.into(),
            cargo_on_file_scope: encode_rule_enabled(value.cargo_on_file_scope),
            cargo_on_diff_scope: encode_rule_enabled(value.cargo_on_diff_scope),
            cargo_test_threads: value.cargo_test_threads,
            run_cargo_doc: encode_rule_enabled(value.run_cargo_doc),
            fail_fast: encode_rule_enabled(value.fail_fast),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WireSourceShapeKind {
    Typescript,
    Rust,
    Python,
    Common,
}
impl From<WireSourceShapeKind> for SourceShapeKind {
    fn from(value: WireSourceShapeKind) -> Self {
        match value {
            WireSourceShapeKind::Typescript => Self::Typescript,
            WireSourceShapeKind::Rust => Self::Rust,
            WireSourceShapeKind::Python => Self::Python,
            WireSourceShapeKind::Common => Self::Common,
        }
    }
}
impl From<SourceShapeKind> for WireSourceShapeKind {
    fn from(value: SourceShapeKind) -> Self {
        match value {
            SourceShapeKind::Typescript => Self::Typescript,
            SourceShapeKind::Rust => Self::Rust,
            SourceShapeKind::Python => Self::Python,
            SourceShapeKind::Common => Self::Common,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireSourceShapePolicy {
    pub roots: Vec<String>,
    pub extensions: Vec<String>,
    pub kind: WireSourceShapeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_classes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_exports: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_functions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_function_lines: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_types: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_nesting_depth: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_branches: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireSourceShapeOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<WireGlob>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub globs: Vec<WireGlob>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_classes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_exports: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_functions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_function_lines: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_types: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_nesting_depth: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_branches: Option<usize>,
}
fn optional_nonzero(
    value: Option<usize>,
    field: &'static str,
) -> Result<Option<NonZeroUsize>, DecodeError> {
    value
        .map(|value| {
            NonZeroUsize::new(value)
                .ok_or_else(|| DecodeError::new(field, "must be greater than zero"))
        })
        .transpose()
}

impl TryFrom<WireSourceShapePolicy> for SourceShapePolicy {
    type Error = DecodeError;

    fn try_from(value: WireSourceShapePolicy) -> Result<Self, Self::Error> {
        Ok(Self {
            roots: value
                .roots
                .into_iter()
                .map(RelPath::try_from)
                .collect::<Result<_, _>>()?,
            extensions: value
                .extensions
                .into_iter()
                .map(ConfigField::from_owned)
                .collect(),
            kind: value.kind.into(),
            max_classes: optional_nonzero(value.max_classes, "maxClasses")?,
            max_exports: optional_nonzero(value.max_exports, "maxExports")?,
            max_functions: optional_nonzero(value.max_functions, "maxFunctions")?,
            max_function_lines: optional_nonzero(value.max_function_lines, "maxFunctionLines")?,
            max_lines: optional_nonzero(value.max_lines, "maxLines")?,
            max_types: optional_nonzero(value.max_types, "maxTypes")?,
            max_nesting_depth: optional_nonzero(value.max_nesting_depth, "maxNestingDepth")?,
            max_branches: optional_nonzero(value.max_branches, "maxBranches")?,
        })
    }
}
impl From<SourceShapePolicy> for WireSourceShapePolicy {
    fn from(value: SourceShapePolicy) -> Self {
        Self {
            roots: value.roots.into_iter().map(String::from).collect(),
            extensions: value
                .extensions
                .into_iter()
                .map(|extension| extension.as_str().to_owned())
                .collect(),
            kind: value.kind.into(),
            max_classes: value.max_classes.map(NonZeroUsize::get),
            max_exports: value.max_exports.map(NonZeroUsize::get),
            max_functions: value.max_functions.map(NonZeroUsize::get),
            max_function_lines: value.max_function_lines.map(NonZeroUsize::get),
            max_lines: value.max_lines.map(NonZeroUsize::get),
            max_types: value.max_types.map(NonZeroUsize::get),
            max_nesting_depth: value.max_nesting_depth.map(NonZeroUsize::get),
            max_branches: value.max_branches.map(NonZeroUsize::get),
        }
    }
}

impl TryFrom<WireSourceShapeOverride> for SourceShapeOverride {
    type Error = DecodeError;

    fn try_from(value: WireSourceShapeOverride) -> Result<Self, Self::Error> {
        Ok(Self {
            path: value.path.map(RelPath::try_from).transpose()?,
            paths: value
                .paths
                .into_iter()
                .map(RelPath::try_from)
                .collect::<Result<_, _>>()?,
            glob: value.glob.map(TryInto::try_into).transpose()?,
            globs: value
                .globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            max_classes: optional_nonzero(value.max_classes, "maxClasses")?,
            max_exports: optional_nonzero(value.max_exports, "maxExports")?,
            max_functions: optional_nonzero(value.max_functions, "maxFunctions")?,
            max_function_lines: optional_nonzero(value.max_function_lines, "maxFunctionLines")?,
            max_lines: optional_nonzero(value.max_lines, "maxLines")?,
            max_types: optional_nonzero(value.max_types, "maxTypes")?,
            max_nesting_depth: optional_nonzero(value.max_nesting_depth, "maxNestingDepth")?,
            max_branches: optional_nonzero(value.max_branches, "maxBranches")?,
        })
    }
}

impl From<SourceShapeOverride> for WireSourceShapeOverride {
    fn from(value: SourceShapeOverride) -> Self {
        Self {
            path: value.path.map(String::from),
            paths: value.paths.into_iter().map(String::from).collect(),
            glob: value.glob.map(Into::into),
            globs: value.globs.into_iter().map(Into::into).collect(),
            max_classes: value.max_classes.map(NonZeroUsize::get),
            max_exports: value.max_exports.map(NonZeroUsize::get),
            max_functions: value.max_functions.map(NonZeroUsize::get),
            max_function_lines: value.max_function_lines.map(NonZeroUsize::get),
            max_lines: value.max_lines.map(NonZeroUsize::get),
            max_types: value.max_types.map(NonZeroUsize::get),
            max_nesting_depth: value.max_nesting_depth.map(NonZeroUsize::get),
            max_branches: value.max_branches.map(NonZeroUsize::get),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireEffectiveConfig {
    pub schema_version: u32,
    pub profile_name: String,
    #[serde(default = "default_supported_platforms")]
    pub supported_platforms: Vec<WirePlatform>,
    #[serde(default)]
    pub harness: WireHarnessConfig,
    #[serde(flatten)]
    pub shape_ownership: WireShapeOwnershipGlobs,
    #[serde(flatten)]
    pub runtime_literal_policy: WireRuntimeLiteralPolicy,
    #[serde(flatten)]
    pub cargo_dependency_policy: WireCargoDependencyPolicy,
    #[serde(flatten)]
    pub rust_scan_scope: WireRustScanScope,
    #[serde(default)]
    pub source_shape_policies: Vec<WireSourceShapePolicy>,
    #[serde(default)]
    pub source_shape_overrides: Vec<WireSourceShapeOverride>,
    #[serde(default)]
    pub architecture_policy_checks: Vec<String>,
    #[serde(default)]
    pub strict_empty_test_trees: bool,
    #[serde(default)]
    pub private_rust_test_module_allowlist: Vec<WirePrivateRustTestModuleAllowlistEntry>,
    #[serde(default)]
    pub ignore_dirs: Vec<String>,
    #[serde(default)]
    pub ignore_file_globs: Vec<WireGlob>,
    #[serde(default)]
    pub boundary_owner_note: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WirePrivateRustTestModuleAllowlistEntry {
    pub owner_file: String,
    pub module_file: String,
    pub module_name: String,
}

impl TryFrom<WireEffectiveConfig> for EffectiveConfig {
    type Error = DecodeError;
    fn try_from(value: WireEffectiveConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            schema_version: std::num::NonZeroU32::new(value.schema_version)
                .ok_or_else(|| DecodeError::new("schemaVersion", "must be greater than zero"))?,
            profile_name: ConfigProfileName::new(value.profile_name)?,
            supported_platforms: value
                .supported_platforms
                .into_iter()
                .map(Into::into)
                .collect(),
            harness: value.harness.into(),
            shape_ownership: value.shape_ownership.try_into()?,
            runtime_literal_policy: value.runtime_literal_policy.try_into()?,
            cargo_dependency_policy: value.cargo_dependency_policy.try_into()?,
            rust_scan_scope: value.rust_scan_scope.try_into()?,
            source_shape_policies: value
                .source_shape_policies
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            source_shape_overrides: value
                .source_shape_overrides
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            architecture_policy_checks: value
                .architecture_policy_checks
                .into_iter()
                .map(ArchitecturePolicyCheck::try_new)
                .collect::<Result<_, _>>()?,
            strict_empty_test_trees: value.strict_empty_test_trees,
            private_rust_test_module_allowlist: value
                .private_rust_test_module_allowlist
                .into_iter()
                .map(|entry| {
                    PrivateRustTestModuleAllowlistEntry::try_new(
                        entry.owner_file.parse()?,
                        entry.module_file.parse()?,
                        entry.module_name,
                    )
                })
                .collect::<Result<_, DecodeError>>()?,
            ignore_dirs: value
                .ignore_dirs
                .into_iter()
                .map(IgnoreDirectorySegment::try_new)
                .collect::<Result<_, _>>()?,
            ignore_file_globs: value
                .ignore_file_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            boundary_owner_note: if value.boundary_owner_note.trim().is_empty() {
                None
            } else {
                Some(PolicyOwner::new(value.boundary_owner_note)?)
            },
        })
    }
}

fn parse_json_value(
    raw: &ConfigJson,
    source_path: &ConfigSource,
    context: &str,
) -> ConfigResult<Value> {
    serde_json::from_str(raw.as_str()).map_err(|error| {
        ConfigLoadError::Parse(DecodeError::new(
            source_path.as_str(),
            format!("{context}: {error}"),
        ))
    })
}

/// Merge a project configuration over its embedded profile at the JSON boundary.
pub(crate) fn resolve_json_layers(
    project_config_json: Option<&ConfigJson>,
    source_path: &ConfigSource,
) -> ConfigResult<EffectiveConfig> {
    let (profile_name, project_value) = match project_config_json {
        None => (
            ConfigProfileName::new("default".to_owned()).map_err(ConfigLoadError::Parse)?,
            None,
        ),
        Some(raw) => {
            let value = parse_json_value(raw, source_path, "invalid JSON")?;
            let profile_name = validate_project_config_shape(source_path, &value)?;
            (profile_name, Some(value))
        }
    };
    let profile_json = embedded_profile_json(&profile_name)?;
    let profile_source = ConfigSource::from_owned("<embedded profile>".to_owned());
    let mut merged = parse_json_value(
        &profile_json,
        &profile_source,
        &format!(
            "embedded profile `{}` failed to parse as JSON",
            profile_name.as_str()
        ),
    )?;
    if let Some(overlay) = &project_value {
        deep_merge_json(&mut merged, overlay);
    }
    decode_effective_config(merged, source_path)
}

/// Resolve one named embedded profile at the JSON boundary.
pub(crate) fn resolve_profile_json(
    profile_name: &ConfigProfileName,
) -> ConfigResult<EffectiveConfig> {
    let profile_json = embedded_profile_json(profile_name)?;
    let source = ConfigSource::from_owned("<embedded profile>".to_owned());
    let value = parse_json_value(
        &profile_json,
        &source,
        &format!(
            "embedded profile `{}` failed to parse as JSON",
            profile_name.as_str()
        ),
    )?;
    decode_effective_config(value, &source)
}

/// Deep-merge object overlays at the JSON boundary; arrays and scalars replace.
pub(crate) fn deep_merge_json(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                match base_map.get_mut(key) {
                    Some(base_value) => deep_merge_json(base_value, overlay_value),
                    None => {
                        base_map.insert(key.clone(), overlay_value.clone());
                    }
                }
            }
        }
        (base_slot, overlay_value) => *base_slot = overlay_value.clone(),
    }
}

fn validate_project_config_shape(
    source_path: &ConfigSource,
    value: &Value,
) -> ConfigResult<ConfigProfileName> {
    let object = value.as_object().ok_or_else(|| {
        ConfigLoadError::Parse(DecodeError::new(
            source_path.as_str(),
            "project config must be a JSON object",
        ))
    })?;
    if !object.contains_key("schemaVersion") {
        return Err(ConfigLoadError::MissingRequiredField {
            path: source_path.clone(),
            field: ConfigField::from_owned("schemaVersion".to_owned()),
        });
    }
    let profile_name = object
        .get("profileName")
        .and_then(Value::as_str)
        .ok_or_else(|| ConfigLoadError::MissingRequiredField {
            path: source_path.clone(),
            field: ConfigField::from_owned("profileName".to_owned()),
        })
        .and_then(|value| ConfigProfileName::new(value.to_owned()).map_err(Into::into))?;
    if !KNOWN_PROFILE_NAMES.contains(&profile_name.as_str()) {
        return Err(ConfigLoadError::UnknownProfile {
            path: source_path.clone(),
            profile_name,
        });
    }
    Ok(profile_name)
}

pub fn decode_json<T: DeserializeOwned>(
    raw: &ConfigJson,
    source_path: &ConfigSource,
    context: &str,
) -> ConfigResult<T> {
    serde_json::from_str(raw.as_str()).map_err(|error| {
        ConfigLoadError::Parse(DecodeError::new(
            source_path.as_str(),
            format!("{context}: {error}"),
        ))
    })
}

pub fn decode_effective_config(
    value: Value,
    source_path: &ConfigSource,
) -> ConfigResult<EffectiveConfig> {
    let wire: WireEffectiveConfig = serde_json::from_value(value).map_err(|error| {
        ConfigLoadError::Parse(DecodeError::new(
            source_path.as_str(),
            format!("resolved config did not decode into WireEffectiveConfig: {error}"),
        ))
    })?;
    wire.try_into().map_err(ConfigLoadError::Parse)
}

pub fn decode_project_config(
    raw: &ConfigJson,
    source_path: &ConfigSource,
) -> ConfigResult<WireProjectConfig> {
    serde_json::from_str(raw.as_str()).map_err(|error| {
        ConfigLoadError::Parse(DecodeError::new(
            source_path.as_str(),
            format!(".enforce/config did not decode into WireProjectConfig: {error}"),
        ))
    })
}

/// Load one project configuration through the sole JSON ingress boundary.
pub fn load_project_config(path: &Path) -> ConfigResult<ProjectConfig> {
    match read_config_json(path)? {
        Some((raw, source)) => decode_project_config(&raw, &source)?
            .try_into()
            .map_err(ConfigLoadError::Parse),
        None => Ok(ProjectConfig::default()),
    }
}

/// Encode a canonical project configuration at the JSON egress boundary.
pub fn encode_project_config(
    config: &ProjectConfig,
    source_path: &ConfigSource,
) -> ConfigResult<ConfigJson> {
    let wire = WireProjectConfig {
        native: config
            .native
            .iter()
            .map(|(tool, tie)| ((*tool).into(), (*tie).into()))
            .collect(),
        policy: config.policy.clone().into(),
    };
    serde_json::to_string_pretty(&wire)
        .map(ConfigJson::from_owned)
        .map_err(|error| {
            ConfigLoadError::Parse(DecodeError::new(
                source_path.as_str(),
                format!("project configuration did not encode into WireProjectConfig: {error}"),
            ))
        })
}

impl From<EffectiveConfig> for WireEffectiveConfig {
    fn from(value: EffectiveConfig) -> Self {
        Self {
            schema_version: value.schema_version.get(),
            profile_name: value.profile_name.as_str().to_owned(),
            supported_platforms: value
                .supported_platforms
                .into_iter()
                .map(Into::into)
                .collect(),
            harness: value.harness.into(),
            shape_ownership: value.shape_ownership.into(),
            runtime_literal_policy: value.runtime_literal_policy.into(),
            cargo_dependency_policy: value.cargo_dependency_policy.into(),
            rust_scan_scope: value.rust_scan_scope.into(),
            source_shape_policies: value
                .source_shape_policies
                .into_iter()
                .map(Into::into)
                .collect(),
            source_shape_overrides: value
                .source_shape_overrides
                .into_iter()
                .map(Into::into)
                .collect(),
            architecture_policy_checks: value
                .architecture_policy_checks
                .into_iter()
                .map(|value| value.as_str().to_owned())
                .collect(),
            strict_empty_test_trees: value.strict_empty_test_trees,
            private_rust_test_module_allowlist: value
                .private_rust_test_module_allowlist
                .into_iter()
                .map(|entry| WirePrivateRustTestModuleAllowlistEntry {
                    owner_file: entry.owner_file().as_str().to_owned(),
                    module_file: entry.module_file().as_str().to_owned(),
                    module_name: entry.module_name().to_owned(),
                })
                .collect(),
            ignore_dirs: value
                .ignore_dirs
                .into_iter()
                .map(|directory| directory.as_str().to_owned())
                .collect(),
            ignore_file_globs: value
                .ignore_file_globs
                .into_iter()
                .map(Into::into)
                .collect(),
            boundary_owner_note: value
                .boundary_owner_note
                .map(|owner| owner.as_str().to_owned())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WireNativeMode {
    Override,
    #[default]
    Augment,
    Both,
}
impl From<WireNativeMode> for NativeMode {
    fn from(value: WireNativeMode) -> Self {
        match value {
            WireNativeMode::Override => Self::Override,
            WireNativeMode::Augment => Self::Augment,
            WireNativeMode::Both => Self::Both,
        }
    }
}
impl From<NativeMode> for WireNativeMode {
    fn from(value: NativeMode) -> Self {
        match value {
            NativeMode::Override => Self::Override,
            NativeMode::Augment => Self::Augment,
            NativeMode::Both => Self::Both,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WireEnforcerScope {
    #[default]
    Scoped,
    WholeRepo,
}
impl From<WireEnforcerScope> for EnforcerScope {
    fn from(value: WireEnforcerScope) -> Self {
        match value {
            WireEnforcerScope::Scoped => Self::Scoped,
            WireEnforcerScope::WholeRepo => Self::WholeRepo,
        }
    }
}
impl From<EnforcerScope> for WireEnforcerScope {
    fn from(value: EnforcerScope) -> Self {
        match value {
            EnforcerScope::Scoped => Self::Scoped,
            EnforcerScope::WholeRepo => Self::WholeRepo,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WireNativeTool {
    Cargo,
    Tsc,
    Ruff,
    Dart,
    Cflint,
}
impl From<WireNativeTool> for NativeTool {
    fn from(value: WireNativeTool) -> Self {
        match value {
            WireNativeTool::Cargo => Self::Cargo,
            WireNativeTool::Tsc => Self::Tsc,
            WireNativeTool::Ruff => Self::Ruff,
            WireNativeTool::Dart => Self::Dart,
            WireNativeTool::Cflint => Self::Cflint,
        }
    }
}
impl From<NativeTool> for WireNativeTool {
    fn from(value: NativeTool) -> Self {
        match value {
            NativeTool::Cargo => Self::Cargo,
            NativeTool::Tsc => Self::Tsc,
            NativeTool::Ruff => Self::Ruff,
            NativeTool::Dart => Self::Dart,
            NativeTool::Cflint => Self::Cflint,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WireNativeTie {
    #[serde(default)]
    pub mode: WireNativeMode,
    #[serde(default)]
    pub scope: WireEnforcerScope,
}
impl From<WireNativeTie> for NativeTie {
    fn from(value: WireNativeTie) -> Self {
        Self {
            mode: value.mode.into(),
            scope: value.scope.into(),
        }
    }
}
impl From<NativeTie> for WireNativeTie {
    fn from(value: NativeTie) -> Self {
        Self {
            mode: value.mode.into(),
            scope: value.scope.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WireRuleToggle {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver: Option<WireWaiver>,
}
fn default_true() -> bool {
    true
}
impl TryFrom<WireRuleToggle> for crate::policy::RuleToggle {
    type Error = DecodeError;

    fn try_from(value: WireRuleToggle) -> Result<Self, Self::Error> {
        Ok(Self {
            enabled: decode_rule_enabled(value.enabled),
            severity: value.severity,
            waiver: value.waiver.map(TryInto::try_into).transpose()?,
        })
    }
}
impl From<crate::policy::RuleToggle> for WireRuleToggle {
    fn from(value: crate::policy::RuleToggle) -> Self {
        Self {
            enabled: encode_rule_enabled(value.enabled),
            severity: value.severity,
            waiver: value.waiver.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WireWaiver {
    pub rule_id: RuleId,
    pub owner: String,
    pub reason: String,
}
impl TryFrom<WireWaiver> for crate::policy::Waiver {
    type Error = DecodeError;

    fn try_from(value: WireWaiver) -> Result<Self, Self::Error> {
        Ok(Self {
            rule_id: value.rule_id,
            owner: PolicyOwner::new(value.owner)?,
            reason: PolicyReason::new(value.reason)?,
        })
    }
}
impl From<crate::policy::Waiver> for WireWaiver {
    fn from(value: crate::policy::Waiver) -> Self {
        Self {
            rule_id: value.rule_id,
            owner: value.owner.as_str().to_owned(),
            reason: value.reason.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WirePolicy {
    #[serde(default)]
    pub owner_globs: Vec<WireGlob>,
    #[serde(default)]
    pub exempt_globs: Vec<WireGlob>,
    #[serde(default)]
    pub allow_regex: Vec<String>,
    #[serde(default)]
    pub skip_cfg_test: bool,
    #[serde(default)]
    pub test_path_globs: Vec<WireGlob>,
    #[serde(default)]
    pub rule_toggles: BTreeMap<RuleId, WireRuleToggle>,
}
impl TryFrom<WirePolicy> for crate::policy::Policy {
    type Error = DecodeError;

    fn try_from(value: WirePolicy) -> Result<Self, Self::Error> {
        Ok(Self {
            owner_globs: value
                .owner_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            exempt_globs: value
                .exempt_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            allow_regex: value
                .allow_regex
                .into_iter()
                .map(RegexPattern::new)
                .collect::<Result<_, _>>()?,
            skip_cfg_test: decode_cfg_test_skipping(value.skip_cfg_test),
            test_path_globs: value
                .test_path_globs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            rule_toggles: value
                .rule_toggles
                .into_iter()
                .map(|(id, toggle)| toggle.try_into().map(|toggle| (id, toggle)))
                .collect::<Result<_, _>>()?,
        })
    }
}
impl From<crate::policy::Policy> for WirePolicy {
    fn from(value: crate::policy::Policy) -> Self {
        Self {
            owner_globs: value.owner_globs.into_iter().map(Into::into).collect(),
            exempt_globs: value.exempt_globs.into_iter().map(Into::into).collect(),
            allow_regex: value
                .allow_regex
                .into_iter()
                .map(|pattern| pattern.as_str().to_owned())
                .collect(),
            skip_cfg_test: encode_cfg_test_skipping(value.skip_cfg_test),
            test_path_globs: value.test_path_globs.into_iter().map(Into::into).collect(),
            rule_toggles: value
                .rule_toggles
                .into_iter()
                .map(|(id, toggle)| (id, toggle.into()))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WireProjectConfig {
    #[serde(default)]
    pub native: BTreeMap<WireNativeTool, WireNativeTie>,
    #[serde(default)]
    pub policy: WirePolicy,
}

#[cfg(test)]
mod harness_config_tests {
    use super::WireHarnessConfig;
    use enforcer_domain::config_types::{HarnessConfig, HarnessRunLimit};

    #[test]
    fn wire_harness_config_preserves_unlimited_and_explicit_zero() -> Result<(), serde_json::Error>
    {
        let wire: WireHarnessConfig = serde_json::from_str(
            r#"{
                "store":"ndjson-duckdb",
                "storageDir":".enforce",
                "maxArtifactBytes":0,
                "maxRuns":null,
                "maxRunsPerTool":0,
                "maxFailedRuns":0,
                "pruneAfterDays":null
            }"#,
        )?;
        let domain = HarnessConfig::from(wire);
        assert_eq!(domain.max_artifact_bytes.get(), 0);
        assert_eq!(domain.max_runs, None);
        assert_eq!(domain.max_runs_per_tool.map(HarnessRunLimit::get), Some(0));
        assert_eq!(domain.max_failed_runs.map(HarnessRunLimit::get), Some(0));

        let encoded = serde_json::to_value(WireHarnessConfig::from(domain))?;
        assert_eq!(encoded["maxRuns"], serde_json::Value::Null);
        assert_eq!(encoded["maxRunsPerTool"], 0);
        assert_eq!(encoded["maxFailedRuns"], 0);
        assert_eq!(encoded["pruneAfterDays"], serde_json::Value::Null);
        Ok(())
    }
}
