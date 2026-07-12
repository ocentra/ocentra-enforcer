//! `EffectiveConfig` — the one typed, total config struct every downstream
//! crate (scan/harness/proof/lang-*) consumes read-only. Built from
//! `enforcer-domain` newtypes plus the field groups enumerated in the arc-03
//! workpack [G5]. No raw file/env reads happen outside `enforcer-config`;
//! everything here is already parsed-at-boundary and total (no `Option`
//! soup for fields the doctrine says must always resolve to a value).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::shape::SourceShapePolicy;

/// Target CI platform a project declares support for. `PORT-1.1` in
/// `enforcer-lang-common` reads `supportedPlatforms` to scope its
/// platform-specific-script check to the project's declared set rather than
/// blanket-failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// Windows CI runner.
    Windows,
    /// macOS CI runner.
    Macos,
    /// Linux CI runner.
    Linux,
}

impl Platform {
    /// The default set when `supportedPlatforms` is absent from config: all
    /// three. Absence must never silently relax the check.
    pub fn all() -> Vec<Platform> {
        vec![Platform::Windows, Platform::Macos, Platform::Linux]
    }
}

/// Policy for `pub use` re-exports repo-wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublicReexportPolicy {
    /// `pub use` re-exports are forbidden.
    Forbid,
    /// `pub use` re-exports are allowed.
    Allow,
}

/// Project policy for tests declared inside production source files.
///
/// `Forbid` keeps validation ownership visible under organized test roots;
/// `Warn` reports the placement without failing a run; `Allow` adopts the
/// in-file unit-test convention deliberately.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InlineTestPolicy {
    /// Inline tests fail the test-placement rule.
    #[default]
    Forbid,
    /// Inline tests produce a non-blocking advisory.
    Warn,
    /// Inline tests are permitted.
    Allow,
}

/// A crate name as it appears in `Cargo.toml` `[dependencies]` /
/// `[package] name`. Not validated beyond non-empty: crate name syntax is
/// cargo's concern, not ours.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CrateName(pub String);

impl CrateName {
    /// View the crate name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CrateName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A glob pattern string (path-scoping glob, not a validated brand — glob
/// syntax is the consuming matcher's concern).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Glob(pub String);

impl Glob {
    /// View the glob pattern.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The `.enforce/` output location/retention settings that arc-15
/// (scan)/arc-17 (proof)/arc-18 (harness) read to decide where and how long
/// to keep run output. `enforcer-config` resolves these settings but never
/// writes to `.enforce/` itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessConfig {
    // NOTE: `Default` below matches the `ocentra-enforcer`/`strict` profile
    // baseline verbatim; `ocentra-parent.json` omits `harness` entirely
    // today, so this default is what a project on that profile actually
    // gets unless it declares its own `harness` override.
    /// Storage backend identifier (e.g. `"ndjson-duckdb"`).
    pub store: String,
    /// Directory (repo-relative) the harness writes run output under.
    pub storage_dir: String,
    /// Max bytes retained per artifact before truncation.
    pub max_artifact_bytes: u64,
    /// Max total runs retained.
    pub max_runs: u32,
    /// Max runs retained per tool.
    pub max_runs_per_tool: u32,
    /// Max failed runs retained.
    pub max_failed_runs: u32,
    /// Prune runs older than this many days.
    pub prune_after_days: u32,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            store: "ndjson-duckdb".to_owned(),
            storage_dir: ".enforce".to_owned(),
            max_artifact_bytes: 8000,
            max_runs: 50,
            max_runs_per_tool: 20,
            max_failed_runs: 20,
            prune_after_days: 14,
        }
    }
}

/// Shape-ownership globs: the six `Vec<Glob>` fields that scope which files
/// own raw string/type boundaries vs. domain-typed facades.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapeOwnershipGlobs {
    /// Globs marking files that legitimately own raw type boundaries
    /// (bin entry points, adapters, FFI, serde, transport).
    #[serde(default)]
    pub raw_type_boundary_globs: Vec<Glob>,
    /// Globs marking facade files (`lib.rs`, `api.rs`, `prelude.rs`).
    #[serde(default)]
    pub facade_file_globs: Vec<Glob>,
    /// Globs marking files allowed to own raw (non-domain-typed) strings.
    #[serde(default)]
    pub raw_string_owner_globs: Vec<Glob>,
    /// Globs marking files that own domain-primitive construction.
    #[serde(default)]
    pub domain_primitive_owner_globs: Vec<Glob>,
    /// Globs marking files that own serialized-domain-primitive fields.
    #[serde(default)]
    pub serialized_domain_owner_globs: Vec<Glob>,
    /// Globs marking files that own runtime string literals.
    #[serde(default)]
    pub runtime_string_owner_globs: Vec<Glob>,
}

/// Runtime-literal policy: whether/where inline runtime string literals are
/// banned, with an escaped-regex allowlist for legitimate exceptions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLiteralPolicy {
    /// Whether the runtime-string-literal ban is enforced at all.
    #[serde(default)]
    pub enforce_runtime_string_literals: bool,
    /// Regex source patterns (escaped, e.g. `env!\(`) exempting matching
    /// lines from the ban. Preserved verbatim on round-trip.
    #[serde(default)]
    pub runtime_string_line_allow_patterns: Vec<String>,
    /// Whether public domain-primitive fields must be serialized (not raw
    /// strings) at their owning boundary.
    #[serde(default)]
    pub enforce_serialized_public_domain_primitives: bool,
    /// Whether workspace-file conventions (root manifest ownership, etc.)
    /// are enforced.
    #[serde(default)]
    pub enforce_workspace_files: bool,
}

/// Cargo / dependency policy: what a crate may depend on, and the one map
/// field (`blockedProtocolDependencies`) banning specific protocol-crate ->
/// runtime-crate edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoDependencyPolicy {
    /// Map of protocol crate name -> the runtime crates it must not depend
    /// on. The only map-shaped field in `EffectiveConfig`.
    #[serde(default)]
    pub blocked_protocol_dependencies: BTreeMap<CrateName, Vec<CrateName>>,
    /// Crates classified as runtime (production) crates.
    #[serde(default)]
    pub runtime_crates: Vec<CrateName>,
    /// Crates classified as test-only (never shipped).
    #[serde(default)]
    pub test_only_crates: Vec<CrateName>,
    /// Git dependencies explicitly allowed despite `allowGitDependencies`.
    #[serde(default)]
    pub allowed_git_dependencies: Vec<CrateName>,
    /// Whether git dependencies are allowed at all.
    #[serde(default)]
    pub allow_git_dependencies: bool,
    /// Whether path dependencies are allowed at all.
    #[serde(default)]
    pub allow_path_dependencies: bool,
    /// Whether `build.rs` build scripts are allowed.
    #[serde(default)]
    pub allow_build_rs: bool,
    /// Whether `unsafe` code is allowed.
    #[serde(default)]
    pub allow_unsafe_code: bool,
    /// Policy on `pub use` re-exports.
    pub public_reexport_policy: PublicReexportPolicy,
    /// Whether `cargo-deny` is required to pass.
    #[serde(default)]
    pub require_cargo_deny: bool,
    /// Whether `cargo-audit` is required to pass.
    #[serde(default)]
    pub require_cargo_audit: bool,
}

/// Rust roots / scan scope: where cargo-aware checks look, and how they
/// scope themselves (whole-file vs. diff-only).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustScanScope {
    /// Repo-relative roots containing Rust sources.
    #[serde(default)]
    pub rust_roots: Vec<String>,
    /// Globs identifying crate roots (each containing its own `Cargo.toml`).
    #[serde(default)]
    pub crate_root_globs: Vec<Glob>,
    /// Globs identifying test files (excluded from production-shape
    /// budgets).
    #[serde(default)]
    pub test_file_globs: Vec<Glob>,
    /// Whether in-file unit tests are forbidden, advisory-only, or allowed.
    #[serde(default)]
    pub inline_test_policy: InlineTestPolicy,
    /// Whether cargo-aware checks run on the whole file scope.
    #[serde(default)]
    pub cargo_on_file_scope: bool,
    /// Whether cargo-aware checks run on diff scope only.
    #[serde(default)]
    pub cargo_on_diff_scope: bool,
    /// Explicit `cargo test` thread count; `None` lets cargo choose.
    #[serde(default)]
    pub cargo_test_threads: Option<std::num::NonZeroUsize>,
    /// Whether `cargo doc` is run as part of the check.
    #[serde(default)]
    pub run_cargo_doc: bool,
    /// Whether the first failing check stops the run immediately.
    #[serde(default)]
    pub fail_fast: bool,
}

/// The fully resolved, total configuration every downstream crate consumes.
/// Produced by layering a project's local overrides over its selected
/// profile's defaults (see `crate::resolve`). Never constructed directly by
/// downstream crates — always via `crate::load` / `crate::resolve`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveConfig {
    /// Config schema version (mechanical `CFG-1.10` requirement: always
    /// present after successful load).
    pub schema_version: u32,
    /// The profile this config resolved against (`strict`, `default`,
    /// `ocentra-enforcer`, or `ocentra-parent`).
    pub profile_name: String,
    /// CI platforms this project declares support for. Defaults to all
    /// three when absent from every layer — never silently relaxed by
    /// omission.
    #[serde(default = "Platform::all")]
    pub supported_platforms: Vec<Platform>,
    /// The `.enforce/` output location/retention settings. Defaults to the
    /// standard `ndjson-duckdb` / `.enforce` layout when a profile omits
    /// `harness` entirely (e.g. `ocentra-parent.json` today).
    #[serde(default)]
    pub harness: HarnessConfig,
    /// Shape-ownership globs (6 fields).
    #[serde(flatten)]
    pub shape_ownership: ShapeOwnershipGlobs,
    /// Runtime-literal policy (4 fields).
    #[serde(flatten)]
    pub runtime_literal_policy: RuntimeLiteralPolicy,
    /// Cargo / dependency policy (11 fields).
    #[serde(flatten)]
    pub cargo_dependency_policy: CargoDependencyPolicy,
    /// Rust roots / scan scope (7 fields incl. `failFast`).
    #[serde(flatten)]
    pub rust_scan_scope: RustScanScope,
    /// Source-shape budgets [G4].
    #[serde(default)]
    pub source_shape_policies: Vec<SourceShapePolicy>,
    /// Directories ignored repo-wide.
    #[serde(default)]
    pub ignore_dirs: Vec<String>,
    /// File globs ignored repo-wide.
    #[serde(default)]
    pub ignore_file_globs: Vec<Glob>,
}

#[cfg(test)]
mod tests {
    use super::{InlineTestPolicy, Platform, RustScanScope};

    #[test]
    fn platform_all_returns_three_platforms_in_stable_order() {
        assert_eq!(
            Platform::all(),
            vec![Platform::Windows, Platform::Macos, Platform::Linux]
        );
    }

    #[test]
    fn platform_wire_form_is_lowercase() -> Result<(), serde_json::Error> {
        assert_eq!(serde_json::to_string(&Platform::Windows)?, "\"windows\"");
        let parsed: Platform = serde_json::from_str("\"linux\"")?;
        assert_eq!(parsed, Platform::Linux);
        Ok(())
    }

    #[test]
    fn inline_test_policy_defaults_to_forbid_and_accepts_each_mode(
    ) -> Result<(), serde_json::Error> {
        let default_scope: RustScanScope = serde_json::from_str("{}")?;
        assert_eq!(default_scope.inline_test_policy, InlineTestPolicy::Forbid);
        for (wire, expected) in [
            ("\"forbid\"", InlineTestPolicy::Forbid),
            ("\"warn\"", InlineTestPolicy::Warn),
            ("\"allow\"", InlineTestPolicy::Allow),
        ] {
            let scope: RustScanScope = serde_json::from_str(&format!(
                "{{\"inlineTestPolicy\":{wire}}}"
            ))?;
            assert_eq!(scope.inline_test_policy, expected);
        }
        Ok(())
    }
}
