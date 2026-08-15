//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Route-plan and native-tool transport DTOs.

use enforcer_config::serde::{WireEnforcerScope, WireNativeMode, WireNativeTool};
use enforcer_domain::config_types::ResolvedNativeTie;
use enforcer_domain::language_types::{
    DetectionMatcher, LanguageId, LiteralDisposition, LiteralProjection,
    LiteralProjectionDisposition, ScanFamilyDisposition,
};
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

/// Reviewed literal-projection disposition for one canonical identity.
///
/// This is separate from structural parser support and supplemental literal
/// routing. A registered value identifies the existing literal projection;
/// it does not claim parser or validator capability.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", deny_unknown_fields, rename_all = "camelCase")]
pub enum CanonicalLiteralDisposition {
    /// The identity participates in a named literal projection row.
    #[serde(rename = "registered")]
    Registered {
        /// Existing literal projection name from the reviewed registry.
        #[serde(rename = "literalName")]
        literal_name: String,
    },
    /// The identity has no current literal projection row.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// The literal projection question does not apply to this identity.
    #[serde(rename = "notApplicable")]
    NotApplicable,
}

impl From<LiteralDisposition> for CanonicalLiteralDisposition {
    fn from(value: LiteralDisposition) -> Self {
        match value {
            LiteralDisposition::Registered { literal_name } => Self::Registered {
                literal_name: literal_name.to_owned(),
            },
            LiteralDisposition::Unsupported => Self::Unsupported,
            LiteralDisposition::NotApplicable => Self::NotApplicable,
        }
    }
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
    /// Dart validator-dispatch family.
    #[serde(rename = "dart")]
    Dart,
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
    /// An existing CLI route-language value is mechanically mapped.
    ///
    /// This variant is valid only for the canonical consumer `cli` field;
    /// the UI field remains explicitly unsupported or not applicable.
    #[serde(rename = "mapped")]
    Mapped {
        /// Existing typed CLI architecture-language value.
        language: CanonicalCliLanguage,
    },
    /// No canonical-identity projection is mechanically proved.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// The consumer question does not apply to this projection.
    #[serde(rename = "notApplicable")]
    NotApplicable,
}

/// Closed wire names for the existing CLI architecture route values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalCliLanguage {
    /// Existing `rust` architecture route value.
    Rust,
    /// Existing `typescript` architecture route value.
    TypeScript,
}

impl serde::Serialize for CanonicalCliLanguage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Rust => "rust",
            Self::TypeScript => "typeScript",
        })
    }
}

impl<'de> serde::Deserialize<'de> for CanonicalCliLanguage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match <String as serde::Deserialize>::deserialize(deserializer)?.as_str() {
            "rust" => Ok(Self::Rust),
            "typeScript" => Ok(Self::TypeScript),
            _ => Err(serde::de::Error::custom(
                "unknown variant for canonical CLI language",
            )),
        }
    }
}

/// Typed canonical-identity native-tool projection.
///
/// `mapped` identifies an existing consumer tool mapping only; it does not
/// claim that the tool can execute successfully for the current repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanonicalNativeToolDisposition {
    /// One existing native-tool identity is mechanically mapped.
    #[serde(rename = "mapped")]
    Mapped {
        /// Existing typed native-tool identity.
        tool: WireNativeTool,
    },
    /// No canonical-identity native-tool mapping is proved.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// The native-tool question does not apply to this projection.
    #[serde(rename = "notApplicable")]
    NotApplicable,
}

/// Typed canonical-identity rule-pack projection.
///
/// A mapped value identifies existing route-selected packs only; it does not
/// claim rule coverage, fact availability, execution, or finding correctness.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", deny_unknown_fields, rename_all = "camelCase")]
pub enum CanonicalRulePackDisposition {
    /// Existing identity-specific packs selected by the current route.
    #[serde(rename = "mapped")]
    Mapped {
        /// Stable, duplicate-free pack order from the existing route mapping.
        packs: Vec<RulePack>,
    },
    /// No identity-specific rule-pack mapping is proved.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// The rule-pack question does not apply to this projection.
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CanonicalConsumerCapabilityProjectionResponse {
    /// Current native-scan family projection, if mechanically mapped.
    pub native_scan: CanonicalScanFamilyDisposition,
    /// Canonical-identity native-tool projection state.
    pub native_tool: CanonicalNativeToolDisposition,
    /// Canonical-identity rule-pack projection state.
    pub rule_packs: CanonicalRulePackDisposition,
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
            native_tool: native_tool_to_wire(value.native_tool()),
            rule_packs: rule_pack_to_wire(value.rule_packs()),
            cli: cli_language_projection_to_wire(value.cli()),
            ui: consumer_disposition_to_wire(value.ui()),
        }
    }
}

/// One identity-preserving result for the opt-in canonical route projection.
///
/// This is path evidence only. It is derived from the typed registry matcher
/// selected by the existing detector and does not add a new matching rule.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", deny_unknown_fields, rename_all = "camelCase")]
pub enum CanonicalDetectionMatcher {
    /// The selected extension matcher.
    Extension { value: String },
    /// The selected exact-basename matcher.
    ExactBasename { value: String },
    /// The selected compound-suffix matcher.
    CompoundSuffix { value: String },
}

impl From<DetectionMatcher> for CanonicalDetectionMatcher {
    fn from(value: DetectionMatcher) -> Self {
        match value {
            DetectionMatcher::Extension(value) => Self::Extension {
                value: value.to_owned(),
            },
            DetectionMatcher::ExactBasename(value) => Self::ExactBasename {
                value: value.to_owned(),
            },
            DetectionMatcher::CompoundSuffix(value) => Self::CompoundSuffix {
                value: value.to_owned(),
            },
        }
    }
}

impl CanonicalDetectionMatcher {
    fn matches_registry_matcher(&self, matcher: DetectionMatcher) -> bool {
        match (self, matcher) {
            (Self::Extension { value }, DetectionMatcher::Extension(expected))
            | (Self::ExactBasename { value }, DetectionMatcher::ExactBasename(expected))
            | (Self::CompoundSuffix { value }, DetectionMatcher::CompoundSuffix(expected)) => {
                value == expected
            }
            _ => false,
        }
    }
}

/// Validate that optional path evidence belongs to the canonical identity's
/// reviewed matcher set. No matcher text is parsed at this boundary.
fn validate_detection_matcher(
    language_id: u16,
    matcher: &CanonicalDetectionMatcher,
) -> Result<(), String> {
    let nonzero = std::num::NonZeroU16::new(language_id)
        .ok_or_else(|| "canonical language id must be non-zero".to_owned())?;
    let id = LanguageId::try_from_registry_index(nonzero)
        .map_err(|_unknown| "canonical language id is outside the reviewed registry".to_owned())?;
    let record = enforcer_syntax::registry::language_registry()
        .iter()
        .find(|record| record.id() == id)
        .ok_or_else(|| "canonical language id is absent from the reviewed registry".to_owned())?;
    if record
        .matchers()
        .iter()
        .copied()
        .any(|expected| matcher.matches_registry_matcher(expected))
    {
        Ok(())
    } else {
        Err("detectionMatcher does not match the canonical registry".to_owned())
    }
}

/// Validate supplemental literal evidence against its typed literal row.
///
/// Supplemental rows have no parser identity, so validation uses the reviewed
/// literal name and matcher slice rather than inventing a canonical ID.
fn validate_supplemental_detection_matcher(
    literal_name: &str,
    matcher: &CanonicalDetectionMatcher,
) -> Result<(), String> {
    let matchers = enforcer_syntax::registry::literal_projections()
        .iter()
        .find_map(|projection| match projection {
            LiteralProjection::Row(
                name,
                LiteralProjectionDisposition::LiteralOnly,
                _,
                matchers,
                _,
            ) if *name == literal_name => Some(*matchers),
            _ => None,
        })
        .ok_or_else(|| "supplemental literal is absent from the reviewed registry".to_owned())?;
    if matchers
        .iter()
        .copied()
        .any(|expected| matcher.matches_registry_matcher(expected))
    {
        Ok(())
    } else {
        Err("detectionMatcher does not match the supplemental literal registry".to_owned())
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
        /// Reviewed literal-projection disposition.
        literal_disposition: CanonicalLiteralDisposition,
        /// Capability state proved by this packet.
        capability: CanonicalCapabilityDisposition,
        /// Typed consumer capability states for this canonical identity.
        consumer_capabilities: CanonicalConsumerCapabilityProjectionResponse,
        /// The selected typed matcher, when path evidence is available.
        detection_matcher: CanonicalDetectionMatcher,
    },
    /// A named literal projection without a canonical parser identity.
    SupplementalLiteral {
        /// Stable supplemental literal identity.
        literal_name: String,
        /// The selected typed matcher from the supplemental literal row.
        detection_matcher: CanonicalDetectionMatcher,
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
        #[serde(rename = "literalDisposition")]
        literal_disposition: CanonicalLiteralDisposition,
        capability: CanonicalCapabilityDisposition,
        #[serde(rename = "scanFamilyDisposition")]
        scan_family_disposition: CanonicalScanFamilyDisposition,
        #[serde(rename = "consumerCapabilities")]
        consumer_capabilities: CanonicalConsumerCapabilityProjectionResponse,
        #[serde(rename = "detectionMatcher")]
        detection_matcher: CanonicalDetectionMatcher,
    },
    #[serde(rename = "supplementalLiteral")]
    SupplementalLiteral {
        #[serde(rename = "literalName")]
        literal_name: &'a str,
        #[serde(rename = "detectionMatcher")]
        detection_matcher: CanonicalDetectionMatcher,
    },
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", deny_unknown_fields, rename_all = "camelCase")]
enum CanonicalLanguageRouteWireOwned {
    #[serde(rename = "canonical")]
    Canonical {
        #[serde(rename = "languageId")]
        language_id: u16,
        #[serde(rename = "canonicalName")]
        canonical_name: String,
        structural: CanonicalStructuralDisposition,
        #[serde(rename = "literalDisposition")]
        literal_disposition: CanonicalLiteralDisposition,
        capability: CanonicalCapabilityDisposition,
        #[serde(rename = "scanFamilyDisposition")]
        scan_family_disposition: CanonicalScanFamilyDisposition,
        #[serde(rename = "consumerCapabilities")]
        consumer_capabilities: CanonicalConsumerCapabilityProjectionResponse,
        #[serde(rename = "detectionMatcher")]
        detection_matcher: CanonicalDetectionMatcher,
    },
    #[serde(rename = "supplementalLiteral")]
    SupplementalLiteral {
        #[serde(rename = "literalName")]
        literal_name: String,
        #[serde(rename = "detectionMatcher")]
        detection_matcher: CanonicalDetectionMatcher,
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
                literal_disposition,
                capability,
                consumer_capabilities,
                detection_matcher,
            } => CanonicalLanguageRouteWire::Canonical {
                language_id: *language_id,
                canonical_name,
                structural: *structural,
                literal_disposition: literal_disposition.clone(),
                capability: *capability,
                consumer_capabilities: consumer_capabilities.clone(),
                detection_matcher: detection_matcher.clone(),
                scan_family_disposition: scan_family_disposition_for_wire(*language_id)
                    .map_err(serde::ser::Error::custom)?,
            },
            Self::SupplementalLiteral {
                literal_name,
                detection_matcher,
            } => CanonicalLanguageRouteWire::SupplementalLiteral {
                literal_name,
                detection_matcher: detection_matcher.clone(),
            },
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
                literal_disposition,
                capability,
                scan_family_disposition,
                consumer_capabilities,
                detection_matcher,
            } => {
                validate_detection_matcher(language_id, &detection_matcher)
                    .map_err(serde::de::Error::custom)?;
                let expected = scan_family_disposition_for_wire(language_id)
                    .map_err(serde::de::Error::custom)?;
                if scan_family_disposition != expected {
                    return Err(serde::de::Error::custom(
                        "scanFamilyDisposition does not match the canonical registry",
                    ));
                }
                let expected_literal =
                    literal_disposition_for_wire(language_id).map_err(serde::de::Error::custom)?;
                if literal_disposition != expected_literal {
                    return Err(serde::de::Error::custom(
                        "literalDisposition does not match the canonical registry",
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
                    literal_disposition,
                    capability,
                    consumer_capabilities,
                    detection_matcher,
                })
            }
            CanonicalLanguageRouteWireOwned::SupplementalLiteral {
                literal_name,
                detection_matcher,
            } => {
                validate_supplemental_detection_matcher(&literal_name, &detection_matcher)
                    .map_err(serde::de::Error::custom)?;
                Ok(Self::SupplementalLiteral {
                    literal_name,
                    detection_matcher,
                })
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
        .map_err(|_unknown| "canonical language id is outside the reviewed registry".to_owned())?;
    let disposition = crate::router::identity::canonical_scan_family_disposition(id)
        .ok_or_else(|| "canonical language id is absent from the reviewed registry".to_owned())?;
    Ok(scan_family_to_wire(disposition))
}

fn literal_disposition_for_wire(language_id: u16) -> Result<CanonicalLiteralDisposition, String> {
    let nonzero = std::num::NonZeroU16::new(language_id)
        .ok_or_else(|| "canonical language id must be non-zero".to_owned())?;
    let id = LanguageId::try_from_registry_index(nonzero)
        .map_err(|_unknown| "canonical language id is outside the reviewed registry".to_owned())?;
    let route = crate::router::identity::canonical_literal_disposition(id)
        .ok_or_else(|| "canonical language id is absent from the reviewed registry".to_owned())?;
    Ok(route.into())
}

fn consumer_capabilities_for_wire(
    language_id: u16,
) -> Result<CanonicalConsumerCapabilityProjectionResponse, String> {
    let nonzero = std::num::NonZeroU16::new(language_id)
        .ok_or_else(|| "canonical language id must be non-zero".to_owned())?;
    let id = LanguageId::try_from_registry_index(nonzero)
        .map_err(|_unknown| "canonical language id is outside the reviewed registry".to_owned())?;
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
        ScanFamilyDisposition::Mapped(enforcer_domain::scan_types::LanguageFamily::Dart) => {
            CanonicalScanFamilyDisposition::Mapped {
                family: CanonicalScanFamily::Dart,
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

fn cli_language_projection_to_wire(
    projection: crate::router::identity::CliLanguageProjection,
) -> CanonicalConsumerDisposition {
    match projection {
        crate::router::identity::CliLanguageProjection::Mapped(
            crate::router::identity::CliLanguage::Rust,
        ) => CanonicalConsumerDisposition::Mapped {
            language: CanonicalCliLanguage::Rust,
        },
        crate::router::identity::CliLanguageProjection::Mapped(
            crate::router::identity::CliLanguage::TypeScript,
        ) => CanonicalConsumerDisposition::Mapped {
            language: CanonicalCliLanguage::TypeScript,
        },
        crate::router::identity::CliLanguageProjection::Unsupported => {
            CanonicalConsumerDisposition::Unsupported
        }
        crate::router::identity::CliLanguageProjection::NotApplicable => {
            CanonicalConsumerDisposition::NotApplicable
        }
    }
}

fn native_tool_to_wire(
    disposition: crate::router::identity::NativeToolProjection,
) -> CanonicalNativeToolDisposition {
    match disposition {
        crate::router::identity::NativeToolProjection::Mapped(tool) => {
            CanonicalNativeToolDisposition::Mapped { tool: tool.into() }
        }
        crate::router::identity::NativeToolProjection::Unsupported => {
            CanonicalNativeToolDisposition::Unsupported
        }
        crate::router::identity::NativeToolProjection::NotApplicable => {
            CanonicalNativeToolDisposition::NotApplicable
        }
    }
}

fn rule_pack_to_wire(
    disposition: crate::router::identity::RulePackProjection,
) -> CanonicalRulePackDisposition {
    match disposition {
        crate::router::identity::RulePackProjection::Mapped(packs) => {
            CanonicalRulePackDisposition::Mapped {
                packs: packs.to_vec(),
            }
        }
        crate::router::identity::RulePackProjection::Unsupported => {
            CanonicalRulePackDisposition::Unsupported
        }
        crate::router::identity::RulePackProjection::NotApplicable => {
            CanonicalRulePackDisposition::NotApplicable
        }
    }
}
