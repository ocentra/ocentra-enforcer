//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
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

/// Structural parse disposition exposed by the opt-in canonical route projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanonicalStructuralDisposition {
    /// The canonical identity has a structural parser route.
    ParseFile,
    /// The canonical identity intentionally has no structural parser route.
    NoParseFile,
}

/// Honest capability disposition exposed by the opt-in canonical route projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanonicalCapabilityDisposition {
    /// No validator or native capability is claimed by this packet.
    Unsupported,
    /// The identity is intentionally not applicable to structural routing.
    NotApplicable,
}

/// One identity-preserving result for the opt-in canonical route projection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanonicalLanguageRouteResponse {
    /// A canonical parser identity retained with honest capability state.
    Canonical {
        /// Stable one-based canonical identity.
        #[serde(rename = "languageId")]
        language_id: u16,
        /// Validated canonical name from the reviewed registry.
        #[serde(rename = "canonicalName")]
        canonical_name: String,
        /// Structural parser disposition.
        structural: CanonicalStructuralDisposition,
        /// Capability state proved by this packet.
        capability: CanonicalCapabilityDisposition,
    },
    /// A named literal projection without a canonical parser identity.
    SupplementalLiteral {
        /// Stable supplemental literal identity.
        #[serde(rename = "literalName")]
        literal_name: String,
    },
    /// No canonical or supplemental matcher applied.
    Unknown,
}
