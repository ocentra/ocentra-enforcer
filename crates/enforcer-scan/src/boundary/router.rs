//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Route-plan and native-tool transport DTOs.

use enforcer_config::serde::{WireEnforcerScope, WireNativeMode, WireNativeTool};
use enforcer_domain::config_types::ResolvedNativeTie;
use enforcer_domain::language_types::{LanguageId, ScanFamilyDisposition};
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

/// Current extension-based validator-dispatch family projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanonicalScanFamilyDisposition {
    /// One existing validator-dispatch family is proved by every matcher.
    #[serde(rename = "mapped")]
    Mapped {
        /// The existing coarse scan family.
        family: CanonicalScanFamily,
    },
    /// No deterministic current family mapping is proved.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// The scan-family question does not apply.
    #[serde(rename = "notApplicable")]
    NotApplicable,
}

/// Closed wire names for the existing scan validator families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanonicalScanFamily {
    /// Rust validator-dispatch family.
    #[serde(rename = "rust")]
    Rust,
    /// TypeScript validator-dispatch family.
    #[serde(rename = "typeScript")]
    TypeScript,
    /// Python validator-dispatch family.
    #[serde(rename = "python")]
    Python,
    /// Terraform validator-dispatch family.
    #[serde(rename = "terraform")]
    Terraform,
    /// YAML/config validator-dispatch family.
    #[serde(rename = "yamlOrConfig")]
    YamlOrConfig,
}

/// Consumer projection state for which no canonical-identity mapping is proved.
///
/// `Unsupported` describes this projection packet only; legacy coarse routes
/// may still map the same repository to a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanonicalConsumerDisposition {
    /// No canonical-identity projection is mechanically proved.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// The consumer question does not apply to this projection.
    #[serde(rename = "notApplicable")]
    NotApplicable,
}

/// Typed consumer-capability values attached to the opt-in canonical route.
///
/// The native-scan field reuses the exact current scan-family mapping. The
/// remaining fields are explicit negative dispositions because their current
/// consumers have no canonical-identity mapping seam.
/// ROUNDTRIP-TEST: `tests/canonical_language_routing.rs::scan_family_wire_rejects_missing_duplicate_unknown_and_mismatched_dispositions`
/// proves this nested response DTO serializes and deserializes without loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CanonicalConsumerCapabilityProjectionResponse {
    /// Current native-scan family projection, if mechanically mapped.
    pub native_scan: CanonicalScanFamilyDisposition,
    /// Canonical-identity native-tool projection state.
    pub native_tool: CanonicalConsumerDisposition,
    /// Canonical-identity rule-pack projection state.
    pub rule_packs: CanonicalConsumerDisposition,
    /// Canonical-identity CLI projection state.
    pub cli: CanonicalConsumerDisposition,
    /// Canonical-identity UI projection state.
    pub ui: CanonicalConsumerDisposition,
}

impl From<crate::router::identity::CanonicalConsumerCapabilities>
    for CanonicalConsumerCapabilityProjectionResponse
{
    fn from(value: crate::router::identity::CanonicalConsumerCapabilities) -> Self {
        Self {
            native_scan: scan_family_to_wire(value.native_scan()),
            native_tool: consumer_disposition_to_wire(value.native_tool()),
            rule_packs: consumer_disposition_to_wire(value.rule_packs()),
            cli: consumer_disposition_to_wire(value.cli()),
            ui: consumer_disposition_to_wire(value.ui()),
        }
    }
}

/// One identity-preserving result for the opt-in canonical route projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalLanguageRouteResponse {
    /// A canonical parser identity retained with honest capability state.
    Canonical {
        /// Stable one-based canonical identity.
        language_id: u16,
        /// Validated canonical name from the reviewed registry.
        canonical_name: String,
        /// Structural parser disposition.
        structural: CanonicalStructuralDisposition,
        /// Capability state proved by this packet.
        capability: CanonicalCapabilityDisposition,
        /// Typed consumer capability states for this canonical identity.
        consumer_capabilities: CanonicalConsumerCapabilityProjectionResponse,
    },
    /// A named literal projection without a canonical parser identity.
    SupplementalLiteral {
        /// Stable supplemental literal identity.
        literal_name: String,
    },
    /// No canonical or supplemental matcher applied.
    Unknown,
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum CanonicalLanguageRouteWire<'a> {
    #[serde(rename = "canonical")]
    Canonical {
        #[serde(rename = "languageId")]
        language_id: u16,
        #[serde(rename = "canonicalName")]
        canonical_name: &'a str,
        structural: CanonicalStructuralDisposition,
        capability: CanonicalCapabilityDisposition,
        #[serde(rename = "scanFamilyDisposition")]
        scan_family_disposition: CanonicalScanFamilyDisposition,
        #[serde(rename = "consumerCapabilities")]
        consumer_capabilities: CanonicalConsumerCapabilityProjectionResponse,
    },
    #[serde(rename = "supplementalLiteral")]
    SupplementalLiteral {
        #[serde(rename = "literalName")]
        literal_name: &'a str,
    },
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum CanonicalLanguageRouteWireOwned {
    #[serde(rename = "canonical")]
    Canonical {
        #[serde(rename = "languageId")]
        language_id: u16,
        #[serde(rename = "canonicalName")]
        canonical_name: String,
        structural: CanonicalStructuralDisposition,
        capability: CanonicalCapabilityDisposition,
        #[serde(rename = "scanFamilyDisposition")]
        scan_family_disposition: CanonicalScanFamilyDisposition,
        #[serde(rename = "consumerCapabilities")]
        consumer_capabilities: CanonicalConsumerCapabilityProjectionResponse,
    },
    #[serde(rename = "supplementalLiteral")]
    SupplementalLiteral {
        #[serde(rename = "literalName")]
        literal_name: String,
    },
    #[serde(rename = "unknown")]
    Unknown,
}

impl serde::Serialize for CanonicalLanguageRouteResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wire = match self {
            Self::Canonical {
                language_id,
                canonical_name,
                structural,
                capability,
                consumer_capabilities,
            } => CanonicalLanguageRouteWire::Canonical {
                language_id: *language_id,
                canonical_name,
                structural: *structural,
                capability: *capability,
                consumer_capabilities: *consumer_capabilities,
                scan_family_disposition: scan_family_disposition_for_wire(*language_id)
                    .map_err(serde::ser::Error::custom)?,
            },
            Self::SupplementalLiteral { literal_name } => {
                CanonicalLanguageRouteWire::SupplementalLiteral { literal_name }
            }
            Self::Unknown => CanonicalLanguageRouteWire::Unknown,
        };
        serde::Serialize::serialize(&wire, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for CanonicalLanguageRouteResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match CanonicalLanguageRouteWireOwned::deserialize(deserializer)? {
            CanonicalLanguageRouteWireOwned::Canonical {
                language_id,
                canonical_name,
                structural,
                capability,
                scan_family_disposition,
                consumer_capabilities,
            } => {
                let expected = scan_family_disposition_for_wire(language_id)
                    .map_err(serde::de::Error::custom)?;
                if scan_family_disposition != expected {
                    return Err(serde::de::Error::custom(
                        "scanFamilyDisposition does not match the canonical registry",
                    ));
                }
                let expected_consumers = consumer_capabilities_for_wire(language_id)
                    .map_err(serde::de::Error::custom)?;
                if consumer_capabilities != expected_consumers {
                    return Err(serde::de::Error::custom(
                        "consumerCapabilities does not match the canonical registry",
                    ));
                }
                Ok(Self::Canonical {
                    language_id,
                    canonical_name,
                    structural,
                    capability,
                    consumer_capabilities,
                })
            }
            CanonicalLanguageRouteWireOwned::SupplementalLiteral { literal_name } => {
                Ok(Self::SupplementalLiteral { literal_name })
            }
            CanonicalLanguageRouteWireOwned::Unknown => Ok(Self::Unknown),
        }
    }
}

fn scan_family_disposition_for_wire(
    language_id: u16,
) -> Result<CanonicalScanFamilyDisposition, String> {
    let nonzero = std::num::NonZeroU16::new(language_id)
        .ok_or_else(|| "canonical language id must be non-zero".to_owned())?;
    let id = LanguageId::try_from_registry_index(nonzero)
        .map_err(|_| "canonical language id is outside the reviewed registry".to_owned())?;
    let disposition = crate::router::identity::canonical_scan_family_disposition(id)
        .ok_or_else(|| "canonical language id is absent from the reviewed registry".to_owned())?;
    Ok(scan_family_to_wire(disposition))
}

fn consumer_capabilities_for_wire(
    language_id: u16,
) -> Result<CanonicalConsumerCapabilityProjectionResponse, String> {
    let nonzero = std::num::NonZeroU16::new(language_id)
        .ok_or_else(|| "canonical language id must be non-zero".to_owned())?;
    let id = LanguageId::try_from_registry_index(nonzero)
        .map_err(|_| "canonical language id is outside the reviewed registry".to_owned())?;
    crate::router::identity::canonical_consumer_capabilities(id)
        .map(Into::into)
        .ok_or_else(|| "canonical language id is absent from the reviewed registry".to_owned())
}

fn scan_family_to_wire(disposition: ScanFamilyDisposition) -> CanonicalScanFamilyDisposition {
    match disposition {
        ScanFamilyDisposition::Mapped(enforcer_domain::scan_types::LanguageFamily::Rust) => {
            CanonicalScanFamilyDisposition::Mapped {
                family: CanonicalScanFamily::Rust,
            }
        }
        ScanFamilyDisposition::Mapped(enforcer_domain::scan_types::LanguageFamily::TypeScript) => {
            CanonicalScanFamilyDisposition::Mapped {
                family: CanonicalScanFamily::TypeScript,
            }
        }
        ScanFamilyDisposition::Mapped(enforcer_domain::scan_types::LanguageFamily::Python) => {
            CanonicalScanFamilyDisposition::Mapped {
                family: CanonicalScanFamily::Python,
            }
        }
        ScanFamilyDisposition::Mapped(enforcer_domain::scan_types::LanguageFamily::Terraform) => {
            CanonicalScanFamilyDisposition::Mapped {
                family: CanonicalScanFamily::Terraform,
            }
        }
        ScanFamilyDisposition::Mapped(
            enforcer_domain::scan_types::LanguageFamily::YamlOrConfig,
        ) => CanonicalScanFamilyDisposition::Mapped {
            family: CanonicalScanFamily::YamlOrConfig,
        },
        ScanFamilyDisposition::Mapped(enforcer_domain::scan_types::LanguageFamily::Unknown)
        | ScanFamilyDisposition::Unsupported => CanonicalScanFamilyDisposition::Unsupported,
        ScanFamilyDisposition::NotApplicable => CanonicalScanFamilyDisposition::NotApplicable,
    }
}

fn consumer_disposition_to_wire(
    disposition: crate::router::identity::ConsumerCapabilityState,
) -> CanonicalConsumerDisposition {
    match disposition {
        crate::router::identity::ConsumerCapabilityState::Unsupported => {
            CanonicalConsumerDisposition::Unsupported
        }
        crate::router::identity::ConsumerCapabilityState::NotApplicable => {
            CanonicalConsumerDisposition::NotApplicable
        }
    }
}
