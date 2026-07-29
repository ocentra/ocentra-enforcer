//! Serde-only catalog DTOs and conversion into pure rule-domain values.
//!
//! BOUNDARY-INVARIANT: parse raw catalog JSON once; no wire DTO leaves this
//! module, and successful decoding yields only validated canonical rule records.
//! boundaryOwnerNote: enforcer-rules owns catalog JSON decoding.
//! Negative invalid, empty, oversized, and malformed catalog coverage is exercised
//! by loader and registry tests.

use std::{collections::BTreeMap, num::NonZeroU32};

use enforcer_domain::{
    config_types::CrateName,
    paths::RelPath,
    rules_types::{
        RuleCatalogJson, RuleDocAnchor, RuleFailureReason, RuleParameter, RuleParameterKey,
        RuleParameterText, RuleParameters, RuleTag, RuleTitle, RuleVersion, ValidatorPath,
    },
    severity::Tier,
};
use serde::Deserialize;
use serde_json::Value;

use super::super::{
    registry::{FixtureRef, RuleRecord, ValidatorRef},
    RuleLoadError, RuleResult,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireFixtureRef {
    fail: String,
    pass: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireValidatorRef {
    crate_name: String,
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireRuleRecord {
    rule_id: enforcer_domain::ids::RuleId,
    version: u32,
    title: String,
    tier: Tier,
    validator: WireValidatorRef,
    fixtures: WireFixtureRef,
    doc_anchor: String,
    // DEFAULT-JUSTIFICATION: omitted tags mean the rule has no grouping labels.
    #[serde(default)]
    tags: Vec<String>,
    // DEFAULT-JUSTIFICATION: omitted params mean the rule has no family-specific parameters.
    #[serde(default)]
    params: Value,
}

fn parameter(value: Value) -> Result<RuleParameter, RuleFailureReason> {
    match value {
        Value::Null => Ok(RuleParameter::Null),
        Value::Bool(value) => Ok(RuleParameter::Boolean(value)),
        Value::Number(value) if value.is_i64() => value
            .as_i64()
            .map(RuleParameter::Integer)
            .ok_or_else(|| boundary_reason("integer JSON number could not be represented")),
        Value::Number(value) if value.is_u64() => value
            .as_u64()
            .map(RuleParameter::Unsigned)
            .ok_or_else(|| boundary_reason("unsigned JSON number could not be represented")),
        Value::Number(value) => RuleParameterText::try_from(value.to_string())
            .map(RuleParameter::Text)
            .map_err(boundary_reason),
        Value::String(value) => RuleParameterText::try_from(value)
            .map(RuleParameter::Text)
            .map_err(boundary_reason),
        Value::Array(values) => values
            .into_iter()
            .map(parameter)
            .collect::<Result<Vec<_>, _>>()
            .map(RuleParameter::List),
        Value::Object(values) => values
            .into_iter()
            .map(|(key, value)| {
                RuleParameterKey::try_from(key)
                    .map_err(boundary_reason)
                    .and_then(|key| parameter(value).map(|value| (key, value)))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map(RuleParameter::Object),
    }
}

fn parameters(value: Value) -> Result<RuleParameters, RuleFailureReason> {
    match value {
        Value::Null => Ok(RuleParameters::default()),
        Value::Object(values) => values
            .into_iter()
            .map(|(key, value)| {
                RuleParameterKey::try_from(key)
                    .map_err(boundary_reason)
                    .and_then(|key| parameter(value).map(|value| (key, value)))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()
            .map(RuleParameters::new),
        _ => Err(boundary_reason("rule params must be a JSON object or null")),
    }
}

fn boundary_reason(error: impl std::fmt::Display) -> RuleFailureReason {
    super::super::boundary_reason(error)
}

impl TryFrom<WireRuleRecord> for RuleRecord {
    type Error = RuleFailureReason;
    fn try_from(value: WireRuleRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            rule_id: value.rule_id,
            version: RuleVersion::try_new(
                NonZeroU32::new(value.version)
                    .ok_or_else(|| boundary_reason("rule version must be nonzero"))?,
            ),
            title: RuleTitle::try_from(value.title).map_err(boundary_reason)?,
            tier: value.tier,
            validator: ValidatorRef {
                crate_name: CrateName::try_from(value.validator.crate_name)
                    .map_err(boundary_reason)?,
                path: ValidatorPath::try_from(value.validator.path).map_err(boundary_reason)?,
            },
            fixtures: FixtureRef {
                fail: RelPath::try_from(value.fixtures.fail).map_err(boundary_reason)?,
                pass: RelPath::try_from(value.fixtures.pass).map_err(boundary_reason)?,
            },
            doc_anchor: RuleDocAnchor::try_from(value.doc_anchor).map_err(boundary_reason)?,
            tags: value
                .tags
                .into_iter()
                .map(|tag| RuleTag::try_from(tag).map_err(boundary_reason))
                .collect::<Result<_, _>>()?,
            params: parameters(value.params)?,
        })
    }
}

/// Decode one catalog document only at the JSON boundary.
pub fn decode_catalog(raw: &RuleCatalogJson) -> RuleResult<Vec<RuleRecord>> {
    let path = enforcer_domain::rules_types::RuleCatalogSource::try_from(
        "catalog JSON boundary".to_owned(),
    )
    .map_err(|error| RuleLoadError::Boundary {
        reason: crate::boundary_reason(error),
    })?;
    let wire: Vec<WireRuleRecord> =
        serde_json::from_str(raw.as_str()).map_err(|error| RuleLoadError::Parse {
            path,
            reason: crate::boundary_reason(error),
        })?;
    wire.into_iter()
        .map(|value| {
            value
                .try_into()
                .map_err(|reason| RuleLoadError::Boundary { reason })
        })
        .collect()
}
