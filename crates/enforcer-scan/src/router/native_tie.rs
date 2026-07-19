//! Native-tool attachment (f05, stage 3b): reads the f03
//! [`ResolvedProjectTie`] to attach the appropriate
//! [`enforcer_config::project_tie::NativeTool`] to each detected language,
//! dispatched (by a downstream consumer, via the arc-18 `enforcer-harness`
//! run-adapters — this module only SELECTS the tool, it does not invoke
//! it).
//!
//! This module never constructs its own native-mode defaults: every
//! [`NativeToolRouteResponse`] carries the tie f03 already resolved (falling back
//! to `Augment`/`Scoped` through [`ResolvedProjectTie::tie`] when the
//! project's `.enforce/config` is silent), so f05 and f03 never disagree
//! about a tool's effective mode.

use crate::boundary::router::NativeToolRouteResponse;
use enforcer_config::project_tie::ResolvedProjectTie;
use enforcer_domain::config_types::NativeTool;
use enforcer_domain::scan_types::DetectedLanguage;

/// ROUNDTRIP-TEST: `tests/router.rs::route_plan_is_data_driven_and_round_trips_through_json`
/// proves this nested DTO round-trips as part of its enclosing route plan.
///
/// A serializable projection of [`ResolvedNativeTie`]
/// (mode + scope, no borrowed state) — the tie type itself does not derive
/// `Serialize`/`Deserialize` (it is an internal resolver output, not a
/// wire type), so [`NativeToolRouteResponse`] carries this flat mirror instead of
/// the tie directly, preserving the exact mode/scope f03 resolved.
/// ROUNDTRIP-TEST: `tests/router.rs::route_plan_is_data_driven_and_round_trips_through_json`
/// proves this nested DTO round-trips as part of its enclosing route plan.
///
/// One native tool attached to the route plan for a detected language,
/// carrying the resolved f03 tie (mode + scope) so consumers know not just
/// WHICH tool but HOW it relates to the enforcer's own checks.
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

/// Resolve the [`NativeToolRouteResponse`]s for one detected language against
/// `tie`. Returns zero entries for a language with no mapped native tool
/// (e.g. [`DetectedLanguage::Go`], for which f03's closed tool set has no
/// variant) — never a fabricated tool.
pub fn native_tools_for(
    language: DetectedLanguage,
    tie: &ResolvedProjectTie,
) -> Vec<NativeToolRouteResponse> {
    match native_tool_for(language) {
        Some(tool) => {
            let resolved = tie.tie(tool);
            vec![NativeToolRouteResponse::from(&resolved)]
        }
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::native_tools_for;
    use enforcer_config::project_tie::{ProjectConfig, ResolvedProjectTie};
    use enforcer_config::serde::{WireNativeMode, WireNativeTool};
    use enforcer_domain::config_types::ConfigSource;
    use enforcer_domain::scan_types::DetectedLanguage;

    fn default_tie() -> Result<ResolvedProjectTie, Box<dyn std::error::Error>> {
        Ok(ResolvedProjectTie::resolve(
            &ProjectConfig::default(),
            &ConfigSource::from_owned("<test>".to_owned()),
        )?)
    }

    #[test]
    fn rust_maps_to_cargo_with_default_augment_tie() -> Result<(), Box<dyn std::error::Error>> {
        let tie = default_tie()?;
        let routes = native_tools_for(DetectedLanguage::Rust, &tie);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].tool, WireNativeTool::Cargo);
        assert_eq!(routes[0].tie.mode, WireNativeMode::Augment);
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
