//! Per-project native-tie config (f03): `.enforce/config` — the
//! serializable schema + loader that ties native tools (`cargo`/`tsc`/
//! `ruff`/`dart`/`CFLint`) to the enforcer via a [`NativeMode`] per tool,
//! plus the declarative [`crate::policy::Policy`]. Consumed by f01 (MCP
//! scan), f02, f05 (native-tie step), and c04 (deny-hook) as a resolved,
//! total policy view — never raw files.
//!
//! # Default posture
//! Absent config, or an absent tool entry, resolves to
//! [`NativeMode::Augment`] scoped to crate/file — native tooling keeps
//! running AND our checks run too, scoped, never whole-repo by default.
//! Whole-repo scope is opt-in only ([`EnforcerScope::WholeRepo`]).

use std::collections::BTreeMap;

use enforcer_core::error::DecodeError;
use serde::{Deserialize, Serialize};

use crate::error::{ConfigLoadError, ConfigResult};
use crate::policy::Policy;

/// How a native tool (`cargo`, `tsc`, `ruff`, `dart`, `CFLint`, ...) relates
/// to the enforcer's own checks for that tool's language surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeMode {
    /// The enforcer replaces the native tool: native does not run, ours
    /// runs instead.
    Override,
    /// The enforcer runs in addition to the native tool: native runs AND
    /// our (scoped) checks also run. This is the default per tool.
    #[default]
    Augment,
    /// Both native and enforcer run, and both are treated as required
    /// (distinct from `Augment` in that neither is optional/advisory).
    Both,
}

/// The scope our own checks run at when [`NativeMode::Augment`] (or
/// `Both`) is selected. Default is [`EnforcerScope::Scoped`]: crate/file
/// scope only. `WholeRepo` must be requested explicitly — it is never the
/// default for any tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EnforcerScope {
    /// Our checks run scoped to the affected crate/file(s) only.
    #[default]
    Scoped,
    /// Our checks run across the whole repository. Opt-in only.
    WholeRepo,
}

/// A recognized native tool identity. Closed set: an unrecognized tool
/// name in `.enforce/config` is a boundary error, not a silently-ignored
/// key (mirrors the `native_mode` boundary-parse requirement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NativeTool {
    /// Rust: `cargo` (build/test/clippy/fmt).
    Cargo,
    /// TypeScript: `tsc`.
    Tsc,
    /// Python: `ruff`.
    Ruff,
    /// Dart: `dart analyze`.
    Dart,
    /// C/C++ lint: `CFLint`.
    Cflint,
}

/// Per-tool tie: the [`NativeMode`] plus the [`EnforcerScope`] our checks
/// run at when not purely `Override`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeTie {
    /// How this tool relates to our own checks.
    #[serde(default)]
    pub mode: NativeMode,
    /// Scope our checks run at for this tool (irrelevant, but retained,
    /// under pure `Override`).
    #[serde(default)]
    pub scope: EnforcerScope,
}

/// The raw, on-disk `.enforce/config` shape: per-tool native ties plus the
/// declarative [`Policy`]. Parsed at the boundary by [`load_project_tie`] /
/// [`parse_project_tie`] — never constructed by downstream crates from raw
/// JSON directly.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectConfig {
    /// Per-tool native ties. A tool absent from this map resolves to the
    /// [`NativeTie::default`] (`Augment`, scoped) — never silently
    /// disabled or widened to whole-repo.
    #[serde(default)]
    pub native: BTreeMap<NativeTool, NativeTie>,
    /// The declarative policy externalization surface.
    #[serde(default)]
    pub policy: Policy,
}

/// One resolved tool entry in a [`ResolvedProjectTie`]: the effective mode
/// and scope, always present (total, no `Option` soup) even for tools the
/// project never mentioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedNativeTie {
    /// The tool this entry resolves.
    pub tool: NativeTool,
    /// Effective mode (defaulted to `Augment` if absent from config).
    pub mode: NativeMode,
    /// Effective scope (defaulted to `Scoped` if absent from config).
    pub scope: EnforcerScope,
}

impl ResolvedNativeTie {
    /// Whether the enforcer's own checks are selected to run for this tool
    /// at all (true for `Augment` and `Both`; false only for pure
    /// `Override`).
    pub fn runs_enforcer_checks(&self) -> bool {
        !matches!(self.mode, NativeMode::Override)
    }

    /// Whether the native tool itself is selected to run (true for
    /// `Augment` and `Both`; false only for pure `Override`, where the
    /// enforcer stands in for it).
    pub fn runs_native_tool(&self) -> bool {
        !matches!(self.mode, NativeMode::Override)
    }
}

/// The resolved, total policy view consumed by c04 (deny-hook), f01 (scan),
/// and f05 (native-tie): every recognized tool has a definite tie, and the
/// declarative [`Policy`] is validated (no silently-disabled rule).
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedProjectTie {
    ties: BTreeMap<NativeTool, ResolvedNativeTie>,
    /// The validated declarative policy.
    pub policy: Policy,
}

/// The full set of recognized native tools, in stable order.
const ALL_NATIVE_TOOLS: [NativeTool; 5] = [
    NativeTool::Cargo,
    NativeTool::Tsc,
    NativeTool::Ruff,
    NativeTool::Dart,
    NativeTool::Cflint,
];

impl ResolvedProjectTie {
    /// Resolve a raw [`ProjectConfig`] into a total view: every recognized
    /// tool gets a [`ResolvedNativeTie`] (defaulted to `Augment`/`Scoped`
    /// when the project's config is silent on it), and the policy is
    /// validated.
    ///
    /// # Errors
    /// Returns [`ConfigLoadError::Parse`] if [`Policy::validate`] rejects a
    /// disabled rule with no attributable waiver.
    pub fn resolve(config: &ProjectConfig, source_path: &str) -> ConfigResult<Self> {
        config
            .policy
            .validate()
            .map_err(|reason| ConfigLoadError::Parse(DecodeError::new(source_path, reason)))?;

        let mut ties = BTreeMap::new();
        for tool in ALL_NATIVE_TOOLS {
            let tie = config.native.get(&tool).copied().unwrap_or_default();
            ties.insert(
                tool,
                ResolvedNativeTie {
                    tool,
                    mode: tie.mode,
                    scope: tie.scope,
                },
            );
        }

        Ok(ResolvedProjectTie {
            ties,
            policy: config.policy.clone(),
        })
    }

    /// The resolved tie for `tool`. [`ResolvedProjectTie::resolve`]
    /// populates every recognized tool, so a lookup miss falls back to the
    /// scoped-`Augment` default rather than panicking — keeping this a
    /// total, non-panicking accessor.
    pub fn tie(&self, tool: NativeTool) -> ResolvedNativeTie {
        self.ties.get(&tool).copied().unwrap_or(ResolvedNativeTie {
            tool,
            mode: NativeMode::default(),
            scope: EnforcerScope::default(),
        })
    }

    /// Iterate every resolved tie in stable tool order.
    pub fn ties(&self) -> impl Iterator<Item = &ResolvedNativeTie> {
        self.ties.values()
    }
}

/// Parse a raw `.enforce/config` JSON string into a [`ProjectConfig`],
/// rejecting malformed input (bad `native_mode`/`native.*.mode`, unknown
/// tool key, unknown top-level key, or a disabled rule with no waiver) with
/// a typed [`ConfigLoadError`] — never a silent default.
///
/// # Errors
/// Returns [`ConfigLoadError::Parse`] for invalid JSON, an unrecognized
/// `NativeMode`/`EnforcerScope`/[`NativeTool`] variant, an unknown field
/// (deny-unknown-fields), or a policy invariant violation.
pub fn parse_project_tie(raw: &str, source_path: &str) -> ConfigResult<ResolvedProjectTie> {
    let config: ProjectConfig = serde_json::from_str(raw).map_err(|e| {
        ConfigLoadError::Parse(DecodeError::new(
            source_path,
            format!(".enforce/config did not decode into ProjectConfig: {e}"),
        ))
    })?;
    ResolvedProjectTie::resolve(&config, source_path)
}

/// Load and resolve `.enforce/config` from `config_path`. Absence of the
/// file resolves to the total default view (every tool `Augment`/`Scoped`,
/// empty policy) — the "zero-config projects work out of the box"
/// invariant, mirroring [`crate::load_project_config`].
///
/// # Errors
/// Returns [`ConfigLoadError::Io`] if the file exists but cannot be read,
/// or [`ConfigLoadError::Parse`] if it is malformed (see
/// [`parse_project_tie`]).
pub fn load_project_tie(config_path: &std::path::Path) -> ConfigResult<ResolvedProjectTie> {
    if !config_path.exists() {
        return ResolvedProjectTie::resolve(&ProjectConfig::default(), "<no .enforce/config>");
    }
    let raw = std::fs::read_to_string(config_path).map_err(|e| ConfigLoadError::Io {
        path: config_path.display().to_string(),
        reason: e.to_string(),
    })?;
    parse_project_tie(&raw, &config_path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        parse_project_tie, EnforcerScope, NativeMode, NativeTool, ProjectConfig, ResolvedProjectTie,
    };
    use serde_json::json;

    #[test]
    fn absent_config_resolves_to_augment_scoped_for_every_tool(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let resolved = ResolvedProjectTie::resolve(&ProjectConfig::default(), "<none>")?;
        for tool in [
            NativeTool::Cargo,
            NativeTool::Tsc,
            NativeTool::Ruff,
            NativeTool::Dart,
            NativeTool::Cflint,
        ] {
            let tie = resolved.tie(tool);
            assert_eq!(tie.mode, NativeMode::Augment);
            assert_eq!(tie.scope, EnforcerScope::Scoped);
            assert!(tie.runs_enforcer_checks());
        }
        Ok(())
    }

    #[test]
    fn valid_config_round_trips_native_mode_and_scope() -> Result<(), Box<dyn std::error::Error>> {
        let raw = json!({
            "native": {
                "cargo": { "mode": "override", "scope": "wholeRepo" },
                "tsc": { "mode": "both" }
            }
        })
        .to_string();
        let resolved = parse_project_tie(&raw, "cfg.json")?;
        let cargo = resolved.tie(NativeTool::Cargo);
        assert_eq!(cargo.mode, NativeMode::Override);
        assert_eq!(cargo.scope, EnforcerScope::WholeRepo);
        assert!(!cargo.runs_native_tool());

        let tsc = resolved.tie(NativeTool::Tsc);
        assert_eq!(tsc.mode, NativeMode::Both);
        assert_eq!(tsc.scope, EnforcerScope::Scoped, "scope omitted -> default");

        // Untouched tool still defaults.
        let ruff = resolved.tie(NativeTool::Ruff);
        assert_eq!(ruff.mode, NativeMode::Augment);
        Ok(())
    }

    #[test]
    fn malformed_native_mode_is_rejected_as_typed_boundary_error() {
        let raw = json!({
            "native": {
                "cargo": { "mode": "yolo" }
            }
        })
        .to_string();
        let outcome = parse_project_tie(&raw, "bad.json");
        assert!(
            outcome.is_err(),
            "unknown native_mode must be rejected, not silently defaulted"
        );
    }

    #[test]
    fn unknown_tool_key_is_rejected() {
        let raw = json!({
            "native": {
                "gofmt": { "mode": "augment" }
            }
        })
        .to_string();
        let outcome = parse_project_tie(&raw, "bad.json");
        assert!(outcome.is_err(), "unrecognized tool name must be rejected");
    }

    #[test]
    fn disabled_rule_without_waiver_fails_at_the_project_tie_boundary() {
        let raw = json!({
            "policy": {
                "ruleToggles": {
                    "RR-1.1": { "enabled": false }
                }
            }
        })
        .to_string();
        let outcome = parse_project_tie(&raw, "bad.json");
        assert!(
            outcome.is_err(),
            "disabling a rule with no waiver must fail to load, not silently succeed"
        );
    }

    #[test]
    fn inline_disable_style_keys_are_not_honored() {
        // An attempt to smuggle an inline-disable-shaped key through
        // `.enforce/config` (not a real field on ProjectConfig) is rejected
        // by deny-unknown-fields, not silently accepted as a suppression.
        let raw = json!({
            "inlineAllow": true
        })
        .to_string();
        let outcome = parse_project_tie(&raw, "bad.json");
        assert!(outcome.is_err());
    }

    #[test]
    fn resolved_tie_serializes_ruleid_keyed_policy_round_trip(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let raw = json!({
            "policy": {
                "ownerGlobs": ["crates/enforcer-config/**"],
                "ruleToggles": {
                    "RR-1.1": {
                        "enabled": false,
                        "waiver": {
                            "ruleId": "RR-1.1",
                            "owner": "platform-team",
                            "reason": "tracked in TICKET-42"
                        }
                    }
                }
            }
        })
        .to_string();
        let resolved = parse_project_tie(&raw, "cfg.json")?;
        assert_eq!(resolved.policy.owner_globs.len(), 1);
        use std::str::FromStr;
        let rule_id = enforcer_domain::ids::RuleId::from_str("RR-1.1")?;
        assert!(!resolved.policy.is_rule_enabled(&rule_id));
        Ok(())
    }
}
