//! Per-project native-tie config (f03): `.enforce/config` â€” the
//! serializable schema + loader that ties native tools (`cargo`/`tsc`/
//! `ruff`/`dart`/`CFLint`) to the enforcer via a [`NativeMode`] per tool,
//! plus the declarative [`crate::policy::Policy`]. Consumed by f01 (MCP
//! scan), f02, f05 (native-tie step), and c04 (deny-hook) as a resolved,
//! total policy view â€” never raw files.
//!
//! # Default posture
//! Absent config, or an absent tool entry, resolves to
//! [`NativeMode::Augment`] scoped to crate/file â€” native tooling keeps
//! running AND our checks run too, scoped, never whole-repo by default.
//! Whole-repo scope is opt-in only ([`EnforcerScope::WholeRepo`]).

use std::collections::BTreeMap;

use crate::error::{ConfigLoadError, ConfigResult};
use crate::policy::Policy;
use crate::serde::{decode_project_config, WireProjectConfig};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::{
    ConfigJson, ConfigSource, EnforcerScope, NativeMode, NativeTie, NativeTool, ResolvedNativeTie,
};

/// How a native tool (`cargo`, `tsc`, `ruff`, `dart`, `CFLint`, ...) relates
/// to the enforcer's own checks for that tool's language surface.
///
/// The scope our own checks run at when [`NativeMode::Augment`] (or
/// `Both`) is selected. Default is [`EnforcerScope::Scoped`]: crate/file
/// scope only. `WholeRepo` must be requested explicitly â€” it is never the
/// default for any tool.
///
/// A recognized native tool identity. Closed set: an unrecognized tool
/// name in `.enforce/config` is a boundary error, not a silently-ignored
/// key (mirrors the `native_mode` boundary-parse requirement).
///
/// Per-tool tie: the [`NativeMode`] plus the [`EnforcerScope`] our checks
/// run at when not purely `Override`.
///
/// The raw, on-disk `.enforce/config` shape: per-tool native ties plus the
/// declarative [`Policy`]. Parsed at the boundary by [`load_project_tie`] /
/// [`parse_project_tie`] â€” never constructed by downstream crates from raw
/// JSON directly.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProjectConfig {
    /// Per-tool native ties. A tool absent from this map resolves to the
    /// [`NativeTie::default`] (`Augment`, scoped) â€” never silently
    /// disabled or widened to whole-repo.
    pub native: BTreeMap<NativeTool, NativeTie>,
    /// The declarative policy externalization surface.
    pub policy: Policy,
}

impl TryFrom<WireProjectConfig> for ProjectConfig {
    type Error = DecodeError;

    fn try_from(value: WireProjectConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            native: value
                .native
                .into_iter()
                .map(|(tool, tie)| (tool.into(), tie.into()))
                .collect(),
            policy: value.policy.try_into()?,
        })
    }
}

/// One resolved tool entry in a [`ResolvedProjectTie`]: the effective mode
/// and scope, always present (total, no `Option` soup) even for tools the
/// project never mentioned.
///
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
    pub fn resolve(config: &ProjectConfig, _source_path: &ConfigSource) -> ConfigResult<Self> {
        config.policy.validate()?;

        let mut ties = BTreeMap::new();
        for tool in ALL_NATIVE_TOOLS {
            let tie = config
                .native
                .get(&tool)
                .copied()
                .map_or_else(NativeTie::default, std::convert::identity);
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
            // CLONE-JUSTIFICATION: The resolved view outlives its borrowed raw config and
            // owns the already-validated policy it exposes to downstream consumers.
            policy: config.policy.clone(),
        })
    }

    /// The resolved tie for `tool`. [`ResolvedProjectTie::resolve`]
    /// populates every recognized tool, so a lookup miss falls back to the
    /// scoped-`Augment` default rather than panicking â€” keeping this a
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
/// a typed [`ConfigLoadError`] â€” never a silent default.
///
/// # Errors
/// Returns [`ConfigLoadError::Parse`] for invalid JSON, an unrecognized
/// `NativeMode`/`EnforcerScope`/[`NativeTool`] variant, an unknown field
/// (deny-unknown-fields), or a policy invariant violation.
pub fn parse_project_tie(
    raw: &ConfigJson,
    source_path: &ConfigSource,
) -> ConfigResult<ResolvedProjectTie> {
    let wire = decode_project_config(raw, source_path)?;
    let config: ProjectConfig = wire.try_into().map_err(ConfigLoadError::Parse)?;
    ResolvedProjectTie::resolve(&config, source_path)
}

/// Load and resolve `.enforce/config` from `config_path`. Absence of the
/// file resolves to the total default view (every tool `Augment`/`Scoped`,
/// empty policy) â€” the "zero-config projects work out of the box"
/// invariant, mirroring [`crate::load_project_config`].
///
/// # Errors
/// Returns [`ConfigLoadError::Io`] if the file exists but cannot be read,
/// or [`ConfigLoadError::Parse`] if it is malformed (see
/// [`parse_project_tie`]).
pub fn load_project_tie(config_path: &std::path::Path) -> ConfigResult<ResolvedProjectTie> {
    match crate::serde::read_config_json(config_path)? {
        Some((raw, source)) => parse_project_tie(&raw, &source),
        None => {
            let source = crate::serde::absent_project_tie_source();
            ResolvedProjectTie::resolve(&ProjectConfig::default(), &source)
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::parse_project_tie;
    use enforcer_domain::config_types::{ConfigJson, ConfigSource, NativeMode, NativeTool};
    use proptest::{prop_assert_eq, proptest};

    proptest! {
        #[test]
        fn parse_project_tie_preserves_generated_recognized_native_modes(
            tool_index in 0_usize..5,
            mode_index in 0_usize..3,
        ) {
            let tools = [
                ("cargo", NativeTool::Cargo),
                ("tsc", NativeTool::Tsc),
                ("ruff", NativeTool::Ruff),
                ("dart", NativeTool::Dart),
                ("cflint", NativeTool::Cflint),
            ];
            let modes = [
                ("override", NativeMode::Override),
                ("augment", NativeMode::Augment),
                ("both", NativeMode::Both),
            ];
            let (tool_name, tool) = tools[tool_index];
            let (mode_name, mode) = modes[mode_index];
            let raw = serde_json::json!({
                "native": {tool_name: {"mode": mode_name}}
            })
            .to_string();

            let resolved = parse_project_tie(
                &ConfigJson::from_owned(raw),
                &ConfigSource::from_owned("generated-project-tie.json".to_owned()),
            )?;

            prop_assert_eq!(resolved.tie(tool).mode, mode);
        }
    }
}
