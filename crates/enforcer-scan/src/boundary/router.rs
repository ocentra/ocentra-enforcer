//! Route-plan and native-tool transport DTOs.

use enforcer_config::serde::{WireEnforcerScope, WireNativeMode, WireNativeTool};
use enforcer_domain::config_types::ResolvedNativeTie;
use enforcer_domain::scan_types::{DetectedLanguage, RouteScope, RulePack};

/// Serializable projection of one resolved native-tool tie.
/// ROUNDTRIP-TEST: `tests/router.rs::route_plan_is_data_driven_and_round_trips_through_json`
/// covers this nested projection through the emitted route plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteTieResponse {
    /// Effective native mode for this tool.
    pub mode: WireNativeMode,
    /// Effective enforcer-check scope for this tool.
    pub scope: WireEnforcerScope,
}

impl From<&ResolvedNativeTie> for RouteTieResponse {
    fn from(value: &ResolvedNativeTie) -> Self {
        Self {
            mode: value.mode.into(),
            scope: value.scope.into(),
        }
    }
}

/// One native tool attached to a route plan.
/// ROUNDTRIP-TEST: `tests/router.rs::route_plan_is_data_driven_and_round_trips_through_json`
/// covers this nested response through the emitted route plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeToolRouteResponse {
    /// Native tool identity.
    pub tool: WireNativeTool,
    /// Effective mode and scope tie.
    pub tie: RouteTieResponse,
}

impl From<&ResolvedNativeTie> for NativeToolRouteResponse {
    fn from(value: &ResolvedNativeTie) -> Self {
        Self {
            tool: value.tool.into(),
            tie: RouteTieResponse::from(value),
        }
    }
}

/// Complete serializable route plan emitted by the router boundary.
/// ROUNDTRIP-TEST: `tests/router.rs::route_plan_is_data_driven_and_round_trips_through_json`
/// verifies the complete response and every nested boundary projection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlanResponse {
    /// Canonical domain scope selected for this plan.
    pub scope: RouteScope,
    /// Detected languages in stable sorted order.
    pub languages: Vec<DetectedLanguage>,
    /// Rule packs selected for those languages.
    pub rule_packs: Vec<RulePack>,
    /// Native tools selected by the resolved project tie.
    pub native_tools: Vec<NativeToolRouteResponse>,
}
