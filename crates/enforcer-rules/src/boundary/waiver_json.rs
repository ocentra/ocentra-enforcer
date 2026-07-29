//! Waiver JSON DTOs; canonical waiver values never derive serde.
//!
//! BOUNDARY-INVARIANT: JSON spelling is converted once into validated narrow
//! waiver values before registry validation runs.
//! boundaryOwnerNote: enforcer-rules owns waiver JSON encoding and decoding.
//! Negative invalid, empty, oversized, and malformed waiver coverage is exercised
//! by waiver unit, property, and integration tests.

use enforcer_domain::{
    ids::RuleId,
    paths::RelPath,
    rules_types::{
        WaiverDocumentJson, WaiverDocumentSource, WaiverExpiryDate, WaiverOwner, WaiverReason,
    },
};
use serde::{Deserialize, Serialize};

use super::super::waiver::{Waiver, WaiverLoadError, WaiverRegistry, WaiverResult};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireWaiver {
    path: String,
    rule_id: RuleId,
    owner: String,
    reason: String,
    // DEFAULT-JUSTIFICATION: omitted expiry means the waiver remains date-unbounded.
    #[serde(default)]
    expires: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireWaiverRegistry {
    // DEFAULT-JUSTIFICATION: an omitted waiver list is an empty registry.
    #[serde(default)]
    waivers: Vec<WireWaiver>,
}

/// Decode waiver JSON at its single ingress boundary.
pub fn decode(
    raw: &WaiverDocumentJson,
    source: &WaiverDocumentSource,
) -> WaiverResult<WaiverRegistry> {
    let wire: WireWaiverRegistry =
        serde_json::from_str(raw.as_str()).map_err(|error| WaiverLoadError::Parse {
            catalog_source: source.clone(),
            reason: super::super::boundary_reason(error),
        })?;
    let waivers = wire
        .waivers
        .into_iter()
        .map(|value| {
            let path =
                RelPath::try_from(value.path).map_err(|error| WaiverLoadError::InvalidPath {
                    detail: super::super::boundary_reason(error),
                })?;
            let rule_id = value.rule_id;
            let owner = WaiverOwner::try_from(value.owner).map_err(|_owner_boundary_error| {
                WaiverLoadError::EmptyOwner {
                    path: path.clone(),
                    rule_id: rule_id.clone(),
                }
            })?;
            let reason =
                WaiverReason::try_from(value.reason).map_err(|_reason_boundary_error| {
                    WaiverLoadError::EmptyReason {
                        path: path.clone(),
                        rule_id: rule_id.clone(),
                    }
                })?;
            let expires = value
                .expires
                .map(WaiverExpiryDate::try_from)
                .transpose()
                .map_err(|error| WaiverLoadError::InvalidExpiry {
                    value: super::super::boundary_reason(error),
                })?;
            Ok(Waiver {
                path,
                rule_id,
                owner,
                reason,
                expires,
            })
        })
        .collect::<WaiverResult<Vec<_>>>()?;
    Ok(WaiverRegistry::new(waivers))
}

/// Encode canonical waivers at their single JSON egress boundary.
pub fn encode(
    registry: &WaiverRegistry,
) -> Result<WaiverDocumentJson, enforcer_domain::boundary::decode_error::DecodeError> {
    let wire = WireWaiverRegistry {
        waivers: registry
            .iter()
            .map(|waiver| WireWaiver {
                path: waiver.path.to_string(),
                rule_id: waiver.rule_id.clone(),
                owner: waiver.owner.as_str().to_owned(),
                reason: waiver.reason.as_str().to_owned(),
                expires: waiver.expires.as_ref().map(ToString::to_string),
            })
            .collect(),
    };
    let raw = serde_json::to_string_pretty(&wire).map_err(|error| {
        enforcer_domain::boundary::decode_error::DecodeError::new(
            "waiverDocumentJson",
            error.to_string(),
        )
    })?;
    WaiverDocumentJson::try_from(raw)
}
