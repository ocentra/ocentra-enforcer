//! Boundary-owned JSON loading and resolution for shape-driven doctrine profiles.
//!
//! The domain values live in `enforcer-domain::doctrine_profile_types` and do
//! not decode wire data. This module owns JSON field names, closed enum
//! conversion, duplicate-row checks, embedded profile lookup, and encoding.
//! It is kept under the boundary module so raw wire values cannot be mistaken
//! for canonical configuration state.
//! BOUNDARY-INVARIANT: every decoded row is converted to closed doctrine values
//! before it leaves this module.
//! NEGATIVE-INPUT-COVERAGE: doctrine_profile integration tests reject unknown
//! families, incompatible language/family pairs, and unexplained disabling.

use enforcer_domain::config_types::{
    ConfigProfileName, ConfigSource, PolicyOwner, PolicyReason, RuleEnabled,
};
use enforcer_domain::doctrine_profile_types::{
    DoctrineFamilyPolicy, DoctrineFamilyRow, DoctrineFrameworkFamily, DoctrineLanguage,
    DoctrineProfile, DoctrineRequirement, DoctrineRequirementPolicyParts, DoctrineRequirementRow,
    DoctrineRuleRow,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use serde::{Deserialize, Serialize};

use crate::error::ConfigLoadError;
use crate::error::ConfigResult;
use enforcer_domain::boundary::decode_error::DecodeError;

const EFFECT_DEFAULT_JSON: &str = include_str!("../../profiles/doctrine/effect-default.json");
const ZOD_JSON: &str = include_str!("../../profiles/doctrine/zod.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireDoctrineProfile {
    schema_version: u32,
    profile_name: String,
    language: String,
    requirements: Vec<WireRequirement>,
    #[serde(default)]
    rule_toggles: Vec<WireRuleToggle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireRequirement {
    requirement: String,
    enabled: bool,
    severity: Severity,
    families: Vec<WireFamilyToggle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireFamilyToggle {
    family: String,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireRuleToggle {
    rule_id: RuleId,
    enabled: bool,
    severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// Decode one doctrine profile at the JSON boundary.
pub fn decode_profile(raw: &str, source: &ConfigSource) -> ConfigResult<DoctrineProfile> {
    let wire: WireDoctrineProfile = serde_json::from_str(raw).map_err(|error| {
        ConfigLoadError::Parse(DecodeError::new(
            source.as_str(),
            format!("doctrine profile JSON did not decode: {error}"),
        ))
    })?;
    wire.try_into().map_err(ConfigLoadError::Parse)
}

/// Encode a validated doctrine profile into its canonical JSON shape.
pub fn encode_profile(profile: &DoctrineProfile) -> ConfigResult<String> {
    serde_json::to_string_pretty(&WireDoctrineProfile::from(profile)).map_err(|error| {
        ConfigLoadError::Parse(DecodeError::new(
            "doctrineProfile",
            format!("doctrine profile did not encode: {error}"),
        ))
    })
}

/// Load one shipped doctrine profile without allowing an unknown name to fall back.
pub fn load_embedded_profile(name: &ConfigProfileName) -> ConfigResult<DoctrineProfile> {
    let raw = match name.as_str() {
        "effect-default" => EFFECT_DEFAULT_JSON,
        "zod" => ZOD_JSON,
        _ => {
            return Err(ConfigLoadError::Parse(DecodeError::new(
                "profileName",
                format!("unknown doctrine profile `{}`", name.as_str()),
            )))
        }
    };
    decode_profile(
        raw,
        &ConfigSource::from_owned("embedded doctrine profile".to_owned()),
    )
}

impl TryFrom<WireDoctrineProfile> for DoctrineProfile {
    type Error = DecodeError;

    fn try_from(value: WireDoctrineProfile) -> Result<Self, Self::Error> {
        ensure_schema_version(value.schema_version)?;
        let profile_name = ConfigProfileName::new(value.profile_name)?;
        let language = DoctrineLanguage::from_wire(&value.language)?;
        let requirements = value
            .requirements
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                let requirement =
                    DoctrineRequirement::from_wire(&row.requirement).map_err(|error| {
                        DecodeError::new(format!("requirements[{index}].requirement"), error.reason)
                    })?;
                let families = row
                    .families
                    .into_iter()
                    .enumerate()
                    .map(|(family_index, family_row)| {
                        let family = DoctrineFrameworkFamily::from_wire(&family_row.family)
                            .map_err(|error| {
                                DecodeError::new(
                                    format!(
                                        "requirements[{index}].families[{family_index}].family"
                                    ),
                                    error.reason,
                                )
                            })?;
                        Ok(DoctrineFamilyRow::from_parts(
                            family,
                            DoctrineFamilyPolicy::from_state(state_from_bool(family_row.enabled)),
                        ))
                    })
                    .collect::<Result<Vec<_>, DecodeError>>()?;
                let owner = row
                    .owner
                    .map(PolicyOwner::try_new)
                    .transpose()
                    .map_err(|error| {
                        DecodeError::new(format!("requirements[{index}].owner"), error.reason)
                    })?;
                let reason =
                    row.reason
                        .map(PolicyReason::try_new)
                        .transpose()
                        .map_err(|error| {
                            DecodeError::new(format!("requirements[{index}].reason"), error.reason)
                        })?;
                DoctrineRequirementRow::try_from_parts(
                    requirement,
                    DoctrineRequirementPolicyParts::from_parts(
                        state_from_bool(row.enabled),
                        row.severity,
                        families,
                        owner,
                        reason,
                    ),
                )
            })
            .collect::<Result<Vec<_>, DecodeError>>()?;

        let rule_toggles = value
            .rule_toggles
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                let owner = row
                    .owner
                    .map(PolicyOwner::try_new)
                    .transpose()
                    .map_err(|error| {
                        DecodeError::new(format!("ruleToggles[{index}].owner"), error.reason)
                    })?;
                let reason =
                    row.reason
                        .map(PolicyReason::try_new)
                        .transpose()
                        .map_err(|error| {
                            DecodeError::new(format!("ruleToggles[{index}].reason"), error.reason)
                        })?;
                DoctrineRuleRow::try_from_parts(
                    row.rule_id,
                    state_from_bool(row.enabled),
                    row.severity,
                    owner,
                    reason,
                )
            })
            .collect::<Result<Vec<_>, DecodeError>>()?;

        DoctrineProfile::try_from_rows(profile_name, language, requirements, rule_toggles)
    }
}

impl From<&DoctrineProfile> for WireDoctrineProfile {
    fn from(value: &DoctrineProfile) -> Self {
        Self {
            schema_version: 1,
            profile_name: value.profile_name().as_str().to_owned(),
            language: value.language().wire_name().to_owned(),
            requirements: value
                .requirements()
                .map(|(requirement, policy)| WireRequirement {
                    requirement: requirement.wire_name().to_owned(),
                    enabled: policy.state() == RuleEnabled::Enabled,
                    severity: policy.severity(),
                    families: policy
                        .family_policies()
                        .map(|(family, family_policy)| WireFamilyToggle {
                            family: family.wire_name().to_owned(),
                            enabled: family_policy.is_enabled(),
                        })
                        .collect(),
                    owner: policy.owner().map(|owner| owner.as_str().to_owned()),
                    reason: policy.reason().map(|reason| reason.as_str().to_owned()),
                })
                .collect(),
            rule_toggles: value
                .rule_toggles()
                .map(|(rule_id, policy)| WireRuleToggle {
                    rule_id: rule_id.clone(),
                    enabled: policy.state() == RuleEnabled::Enabled,
                    severity: policy.severity(),
                    owner: policy.owner().map(|owner| owner.as_str().to_owned()),
                    reason: policy.reason().map(|reason| reason.as_str().to_owned()),
                })
                .collect(),
        }
    }
}

fn state_from_bool(enabled: bool) -> RuleEnabled {
    if enabled {
        RuleEnabled::Enabled
    } else {
        RuleEnabled::Disabled
    }
}

fn ensure_schema_version(version: u32) -> Result<(), DecodeError> {
    (version == 1).then_some(()).ok_or_else(|| {
        DecodeError::new(
            "schemaVersion",
            format!("unsupported doctrine profile schema version {version}"),
        )
    })
}
