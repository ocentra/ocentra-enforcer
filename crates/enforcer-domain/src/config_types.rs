//! Canonical configuration domain values.
//!
//! JSON, environment, and file representations belong to `enforcer-config`
//! boundary DTOs, which convert into these values after validation.

use std::collections::BTreeMap;

use crate::boundary::decode_error::DecodeError;
use crate::paths::RelPath;
use crate::scan_types::IgnoreDirectorySegment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[doc = "Canonical domain representation for Platform."]
pub enum Platform {
    Windows,
    Macos,
    Linux,
}

impl Platform {
    #[must_use]
    #[doc = "The all operation for this canonical domain value."]
    pub fn all() -> Vec<Self> {
        vec![Self::Windows, Self::Macos, Self::Linux]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for PublicReexportPolicy."]
pub enum PublicReexportPolicy {
    Forbid,
    Allow,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[doc = "Canonical domain representation for InlineTestPolicy."]
pub enum InlineTestPolicy {
    #[default]
    Forbid,
    Warn,
    Allow,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for CrateName."]
pub struct CrateName(String);
impl CrateName {
    /// Construct a Cargo package name, rejecting invalid spelling.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(DecodeError::new(
                "crateName",
                "must be a non-empty ASCII Cargo package name",
            ));
        }
        Ok(Self(value))
    }

    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for CrateName {
    type Error = DecodeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}
impl std::str::FromStr for CrateName {
    type Err = DecodeError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: CrateName owns validated text after parsing a borrowed CLI/config token.
        Self::try_new(value.to_owned())
    }
}
impl std::fmt::Display for CrateName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for Glob."]
pub struct Glob(String);
impl Glob {
    /// Construct a glob, rejecting invalid blank input.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        (!value.trim().is_empty())
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("glob", "must not be empty"))
    }
    pub fn new(value: String) -> Result<Self, DecodeError> {
        Self::try_new(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for PolicyOwner."]
pub struct PolicyOwner(String);
impl PolicyOwner {
    /// Construct an owner, rejecting invalid blank input.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        (!value.trim().is_empty())
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("policyOwner", "must not be empty"))
    }
    pub fn new(value: String) -> Result<Self, DecodeError> {
        Self::try_new(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for PolicyReason."]
pub struct PolicyReason(String);
impl PolicyReason {
    /// Construct a reason, rejecting invalid blank input.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        (!value.trim().is_empty())
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("policyReason", "must not be empty"))
    }
    pub fn new(value: String) -> Result<Self, DecodeError> {
        Self::try_new(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether one policy rule remains enabled after configuration is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for RuleEnabled."]
pub enum RuleEnabled {
    Disabled,
    Enabled,
}
impl RuleEnabled {
    #[must_use]
    pub const fn enabled() -> Self {
        Self::Enabled
    }
    #[must_use]
    pub const fn disabled() -> Self {
        Self::Disabled
    }
}
impl Default for RuleEnabled {
    fn default() -> Self {
        Self::enabled()
    }
}

/// Whether policy explicitly excludes `cfg(test)` surfaces from scoped checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for CfgTestSkipping."]
pub enum CfgTestSkipping {
    #[default]
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for RegexPattern."]
pub struct RegexPattern(String);
impl RegexPattern {
    /// Construct a regex pattern, rejecting invalid blank input.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        (!value.trim().is_empty())
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("regexPattern", "must not be empty"))
    }
    pub fn new(value: String) -> Result<Self, DecodeError> {
        Self::try_new(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ConfigSource."]
pub struct ConfigSource(String);
impl ConfigSource {
    #[doc = "Retain an owned configuration source label."]
    pub fn from_owned(value: String) -> Self {
        Self(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ConfigField."]
pub struct ConfigField(String);
impl ConfigField {
    #[doc = "Retain an owned configuration field label."]
    pub fn from_owned(value: String) -> Self {
        Self(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ConfigProfileName."]
pub struct ConfigProfileName(String);
impl ConfigProfileName {
    /// Construct a profile name, rejecting invalid blank input.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        (!value.trim().is_empty())
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("configProfileName", "must not be empty"))
    }
    pub fn new(value: String) -> Result<Self, DecodeError> {
        Self::try_new(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ConfigErrorReason."]
pub struct ConfigErrorReason(String);
impl ConfigErrorReason {
    #[doc = "Retain an owned configuration error reason."]
    pub fn from_owned(value: String) -> Self {
        Self(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ConfigEnvironmentVariable."]
pub struct ConfigEnvironmentVariable(String);
impl ConfigEnvironmentVariable {
    /// Construct an environment-variable name, rejecting invalid blank input.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        (!value.trim().is_empty())
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("configEnvironmentVariable", "must not be empty"))
    }
    pub fn new(value: String) -> Result<Self, DecodeError> {
        Self::try_new(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ConfigEnvironmentValue."]
pub struct ConfigEnvironmentValue(String);
impl ConfigEnvironmentValue {
    #[doc = "Retain an owned environment value."]
    pub fn from_owned(value: String) -> Self {
        Self(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
    #[doc = "The into_string operation for this canonical domain value."]
    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ConfigJson."]
pub struct ConfigJson(String);
impl ConfigJson {
    #[doc = "Retain an owned JSON document."]
    pub fn from_owned(value: String) -> Self {
        Self(value)
    }
    #[must_use]
    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! zero_valid_harness_value {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            /// Retain an explicit zero-or-greater harness configuration value.
            #[must_use]
            pub const fn from_value(value: u64) -> Self {
                Self(value)
            }

            /// Return the configured primitive at the adapter seam.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}

zero_valid_harness_value!(
    /// Maximum artifact bytes returned by a harness query.
    HarnessArtifactByteLimit
);
zero_valid_harness_value!(
    /// Explicit run-retention count. Zero is a valid limit.
    HarnessRunLimit
);
zero_valid_harness_value!(
    /// Explicit retention age in days. Zero is a valid limit.
    HarnessRetentionDays
);

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for HarnessConfig."]
pub struct HarnessConfig {
    pub store: ConfigSource,
    pub storage_dir: ConfigSource,
    pub max_artifact_bytes: HarnessArtifactByteLimit,
    /// `None` means unlimited; `Some(0)` means retain no ordinary runs.
    pub max_runs: Option<HarnessRunLimit>,
    /// `None` means unlimited; `Some(0)` means retain no runs per tool.
    pub max_runs_per_tool: Option<HarnessRunLimit>,
    /// `None` means unlimited; `Some(0)` means retain no failed runs.
    pub max_failed_runs: Option<HarnessRunLimit>,
    /// `None` means unlimited; `Some(0)` prunes any positively-aged run.
    pub prune_after_days: Option<HarnessRetentionDays>,
}
impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
            store: ConfigSource::from_owned("ndjson-duckdb".to_owned()),
            // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
            storage_dir: ConfigSource::from_owned(".enforce".to_owned()),
            max_artifact_bytes: HarnessArtifactByteLimit::from_value(8000),
            max_runs: Some(HarnessRunLimit::from_value(50)),
            max_runs_per_tool: Some(HarnessRunLimit::from_value(20)),
            max_failed_runs: Some(HarnessRunLimit::from_value(20)),
            prune_after_days: Some(HarnessRetentionDays::from_value(14)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for ShapeOwnershipGlobs."]
pub struct ShapeOwnershipGlobs {
    pub raw_type_boundary_globs: Vec<Glob>,
    pub facade_file_globs: Vec<Glob>,
    pub raw_string_owner_globs: Vec<Glob>,
    pub domain_primitive_owner_globs: Vec<Glob>,
    pub serialized_domain_owner_globs: Vec<Glob>,
    pub runtime_string_owner_globs: Vec<Glob>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for RuntimeLiteralPolicy."]
pub struct RuntimeLiteralPolicy {
    pub enforce_runtime_string_literals: RuleEnabled,
    pub runtime_string_line_allow_patterns: Vec<RegexPattern>,
    pub enforce_serialized_public_domain_primitives: RuleEnabled,
    pub enforce_workspace_files: RuleEnabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for CargoDependencyPolicy."]
pub struct CargoDependencyPolicy {
    pub blocked_protocol_dependencies: BTreeMap<CrateName, Vec<CrateName>>,
    pub runtime_crates: Vec<CrateName>,
    pub test_only_crates: Vec<CrateName>,
    pub allowed_git_dependencies: Vec<CrateName>,
    pub allowed_build_rs_paths: Vec<RelPath>,
    pub allow_git_dependencies: RuleEnabled,
    pub allow_path_dependencies: RuleEnabled,
    pub allow_build_rs: RuleEnabled,
    pub allow_unsafe_code: RuleEnabled,
    pub public_reexport_policy: PublicReexportPolicy,
    pub require_cargo_deny: RuleEnabled,
    pub require_cargo_audit: RuleEnabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for RustScanScope."]
pub struct RustScanScope {
    pub rust_roots: Vec<RelPath>,
    pub crate_root_globs: Vec<Glob>,
    pub test_file_globs: Vec<Glob>,
    pub inline_test_policy: InlineTestPolicy,
    pub cargo_on_file_scope: RuleEnabled,
    pub cargo_on_diff_scope: RuleEnabled,
    pub cargo_test_threads: Option<std::num::NonZeroUsize>,
    pub run_cargo_doc: RuleEnabled,
    pub fail_fast: RuleEnabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for SourceShapeKind."]
pub enum SourceShapeKind {
    Typescript,
    Rust,
    Python,
    Common,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for SourceShapePolicy."]
pub struct SourceShapePolicy {
    pub roots: Vec<RelPath>,
    pub extensions: Vec<ConfigField>,
    pub kind: SourceShapeKind,
    pub max_classes: Option<std::num::NonZeroUsize>,
    pub max_exports: Option<std::num::NonZeroUsize>,
    pub max_functions: Option<std::num::NonZeroUsize>,
    pub max_function_lines: Option<std::num::NonZeroUsize>,
    pub max_lines: Option<std::num::NonZeroUsize>,
    pub max_types: Option<std::num::NonZeroUsize>,
    pub max_nesting_depth: Option<std::num::NonZeroUsize>,
    pub max_branches: Option<std::num::NonZeroUsize>,
}

/// A path-selected, explicit source-shape budget adjustment.  This is typed
/// policy data, not a waiver: it can only change the same numeric dimensions
/// a base [`SourceShapePolicy`] exposes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for SourceShapeOverride."]
pub struct SourceShapeOverride {
    pub path: Option<RelPath>,
    pub paths: Vec<RelPath>,
    pub glob: Option<Glob>,
    pub globs: Vec<Glob>,
    pub max_classes: Option<std::num::NonZeroUsize>,
    pub max_exports: Option<std::num::NonZeroUsize>,
    pub max_functions: Option<std::num::NonZeroUsize>,
    pub max_function_lines: Option<std::num::NonZeroUsize>,
    pub max_lines: Option<std::num::NonZeroUsize>,
    pub max_types: Option<std::num::NonZeroUsize>,
    pub max_nesting_depth: Option<std::num::NonZeroUsize>,
    pub max_branches: Option<std::num::NonZeroUsize>,
}

/// One configured member of the architecture-policy aggregate.  The string is
/// validated at the configuration boundary; aliases are normalized by the
/// aggregate executor so configuration order remains meaningful for display
/// while duplicate work is never run twice.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical configured architecture-policy check name."]
pub struct ArchitecturePolicyCheck(String);
impl ArchitecturePolicyCheck {
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(DecodeError::new(
                "architecturePolicyChecks",
                "entries must be non-empty printable text",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateRustTestModuleAllowlistEntry {
    owner_file: crate::paths::RelPath,
    module_file: crate::paths::RelPath,
    // BRAND-INVARIANT: `try_new` accepts only a Rust identifier ending in
    // `_private_tests` that exactly matches the module file basename.
    module_name: String,
}

impl PrivateRustTestModuleAllowlistEntry {
    pub fn try_new(
        owner_file: crate::paths::RelPath,
        module_file: crate::paths::RelPath,
        module_name: String,
    ) -> Result<Self, DecodeError> {
        let valid_name = module_name.ends_with("_private_tests")
            && module_name
                .chars()
                .next()
                .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
            && module_name
                .chars()
                .all(|character| character == '_' || character.is_ascii_alphanumeric());
        let same_directory = owner_file
            .as_str()
            .rsplit_once('/')
            .map(|(directory, _)| directory)
            == module_file
                .as_str()
                .rsplit_once('/')
                .map(|(directory, _)| directory);
        let expected_file = format!("{module_name}.rs");
        if !valid_name || !same_directory || !module_file.as_str().ends_with(&expected_file) {
            return Err(DecodeError::new("privateRustTestModuleAllowlist", "entries require same-directory Rust owner/module paths and a *_private_tests module name"));
        }
        Ok(Self {
            owner_file,
            module_file,
            module_name,
        })
    }
    pub fn owner_file(&self) -> &crate::paths::RelPath {
        &self.owner_file
    }
    pub fn module_file(&self) -> &crate::paths::RelPath {
        &self.module_file
    }
    pub fn module_name(&self) -> &str {
        &self.module_name
    }
}

#[derive(Debug, Clone, PartialEq)]
#[doc = "Canonical domain representation for EffectiveConfig."]
pub struct EffectiveConfig {
    pub schema_version: std::num::NonZeroU32,
    pub profile_name: ConfigProfileName,
    pub supported_platforms: Vec<Platform>,
    pub harness: HarnessConfig,
    pub shape_ownership: ShapeOwnershipGlobs,
    pub runtime_literal_policy: RuntimeLiteralPolicy,
    pub cargo_dependency_policy: CargoDependencyPolicy,
    pub rust_scan_scope: RustScanScope,
    pub source_shape_policies: Vec<SourceShapePolicy>,
    pub source_shape_overrides: Vec<SourceShapeOverride>,
    pub architecture_policy_checks: Vec<ArchitecturePolicyCheck>,
    pub strict_empty_test_trees: bool,
    pub private_rust_test_module_allowlist: Vec<PrivateRustTestModuleAllowlistEntry>,
    pub ignore_dirs: Vec<IgnoreDirectorySegment>,
    pub ignore_file_globs: Vec<Glob>,
    pub boundary_owner_note: Option<PolicyOwner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for NativeMode."]
pub enum NativeMode {
    Override,
    #[default]
    Augment,
    Both,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for EnforcerScope."]
pub enum EnforcerScope {
    #[default]
    Scoped,
    WholeRepo,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[doc = "Canonical domain representation for NativeTool."]
pub enum NativeTool {
    Cargo,
    Tsc,
    Ruff,
    Dart,
    Cflint,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "Canonical domain representation for NativeTie."]
pub struct NativeTie {
    pub mode: NativeMode,
    pub scope: EnforcerScope,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ResolvedNativeTie."]
pub struct ResolvedNativeTie {
    pub tool: NativeTool,
    pub mode: NativeMode,
    pub scope: EnforcerScope,
}
