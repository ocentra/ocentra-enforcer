//! Native-tool attachment (f05, stage 3b): reads the f03
//! [`ResolvedProjectTie`] to attach the appropriate
//! [`enforcer_config::project_tie::NativeTool`] to each detected language,
//! dispatched (by a downstream consumer, via the arc-18 `enforcer-harness`
//! run-adapters — this module only SELECTS the tool, it does not invoke
//! it).
//!
//! This module never constructs its own native-mode defaults: every
//! [`NativeToolRoute`] carries the tie f03 already resolved (falling back
//! to `Augment`/`Scoped` through [`ResolvedProjectTie::tie`] when the
//! project's `.enforce/config` is silent), so f05 and f03 never disagree
//! about a tool's effective mode.

use enforcer_config::project_tie::{EnforcerScope, NativeMode, NativeTool, ResolvedProjectTie};
use serde::{Deserialize, Serialize};

use super::detect::DetectedLanguage;

/// A serializable projection of `enforcer_config::project_tie::ResolvedNativeTie`
/// (mode + scope, no borrowed state) — the tie type itself does not derive
/// `Serialize`/`Deserialize` (it is an internal resolver output, not a
/// wire type), so [`NativeToolRoute`] carries this flat mirror instead of
/// the tie directly, preserving the exact mode/scope f03 resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteTie {
    /// Effective native mode for this tool.
    pub mode: NativeMode,
    /// Effective enforcer-checks scope for this tool.
    pub scope: EnforcerScope,
}

/// One native tool attached to the route plan for a detected language,
/// carrying the resolved f03 tie (mode + scope) so consumers know not just
/// WHICH tool but HOW it relates to the enforcer's own checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeToolRoute {
    /// The native tool identity (`cargo`, `tsc`, `ruff`, `dart`, `CFLint`).
    pub tool: NativeTool,
    /// The f03-resolved tie (mode + scope) for this tool.
    pub tie: RouteTie,
}

/// The native tool a [`DetectedLanguage`] maps to, if any. `Dart`/`Go`/
/// `Cfml` map to `dart`/no native tool landed/`CFLint` respectively; `Go`
/// has no [`NativeTool`] variant yet (f03's closed set is `cargo, tsc,
/// ruff, dart, cflint`) so it selects no native tool — never a false
/// invented one.
fn native_tool_for(language: DetectedLanguage) -> Option<NativeTool> {
    match language {
        DetectedLanguage::Rust => Some(NativeTool::Cargo),
        DetectedLanguage::TypeScript => Some(NativeTool::Tsc),
        DetectedLanguage::Python => Some(NativeTool::Ruff),
        DetectedLanguage::Dart => Some(NativeTool::Dart),
        DetectedLanguage::Cfml => Some(NativeTool::Cflint),
        DetectedLanguage::Go | DetectedLanguage::Other => None,
    }
}

/// Resolve the [`NativeToolRoute`]s for one detected language against
/// `tie`. Returns zero entries for a language with no mapped native tool
/// (e.g. [`DetectedLanguage::Go`], for which f03's closed tool set has no
/// variant) — never a fabricated tool.
pub fn native_tools_for(
    language: DetectedLanguage,
    tie: &ResolvedProjectTie,
) -> Vec<NativeToolRoute> {
    match native_tool_for(language) {
        Some(tool) => {
            let resolved = tie.tie(tool);
            vec![NativeToolRoute {
                tool,
                tie: RouteTie {
                    mode: resolved.mode,
                    scope: resolved.scope,
                },
            }]
        }
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{native_tools_for, DetectedLanguage};
    use enforcer_config::project_tie::{NativeMode, NativeTool, ProjectConfig, ResolvedProjectTie};

    fn default_tie() -> Result<ResolvedProjectTie, Box<dyn std::error::Error>> {
        Ok(ResolvedProjectTie::resolve(
            &ProjectConfig::default(),
            "<test>",
        )?)
    }

    #[test]
    fn rust_maps_to_cargo_with_default_augment_tie() -> Result<(), Box<dyn std::error::Error>> {
        let tie = default_tie()?;
        let routes = native_tools_for(DetectedLanguage::Rust, &tie);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].tool, NativeTool::Cargo);
        assert_eq!(routes[0].tie.mode, NativeMode::Augment);
        Ok(())
    }

    #[test]
    fn go_has_no_mapped_native_tool_in_the_closed_set() -> Result<(), Box<dyn std::error::Error>> {
        let tie = default_tie()?;
        assert!(native_tools_for(DetectedLanguage::Go, &tie).is_empty());
        Ok(())
    }

    #[test]
    fn other_language_selects_no_native_tool() -> Result<(), Box<dyn std::error::Error>> {
        let tie = default_tie()?;
        assert!(native_tools_for(DetectedLanguage::Other, &tie).is_empty());
        Ok(())
    }
}
