//! Typed wire boundary for the CP09 API-security manifest packets.
//!
//! BOUNDARY-INVARIANT: this module decodes only supplied static JSON records;
//! it never calls an endpoint, browser, scanner, fuzzer, identity provider, or
//! production API.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_api_security_manifest.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_api_security_manifest.rs

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const ENUMERATION_SKILL: &str = "detecting-api-enumeration-attacks";
const BOPLA_SKILL: &str = "detecting-broken-object-property-level-authorization";
const SHADOW_SKILL: &str = "detecting-shadow-api-endpoints";
const INJECTION_SKILL: &str = "exploiting-api-injection-vulnerabilities";
const BFLA_SKILL: &str = "exploiting-broken-function-level-authorization";

// BRAND-INVARIANT: private wire fields are decoded only through serde and
// accepted by `is_valid` after the schema and field predicates below pass.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestWire {
    schema_version: u8,
    bundle_id: String,
    owner: String,
    scope: String,
    evidence: Vec<EvidenceWire>,
    records: Vec<RecordWire>,
}

// BRAND-INVARIANT: evidence roles are non-empty supplied references only.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceWire {
    kind: String,
    reference: String,
}

// BRAND-INVARIANT: the closed record table below binds each serialized record
// kind to one approved API-security catalog identity and required fields.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecordWire {
    kind: String,
    skill_id: Option<String>,
    subject_ref: Option<String>,
    route_ref: Option<String>,
    request_ref: Option<String>,
    response_ref: Option<String>,
    authorization_ref: Option<String>,
    rate_policy_ref: Option<String>,
    observation_ref: Option<String>,
    schema_ref: Option<String>,
    property_ref: Option<String>,
    identity_ref: Option<String>,
    role_ref: Option<String>,
    version_ref: Option<String>,
    owner_ref: Option<String>,
    documentation_ref: Option<String>,
    parser_ref: Option<String>,
    parameter_ref: Option<String>,
    scope_ref: Option<String>,
    function_ref: Option<String>,
    stop_condition: Option<String>,
    review_ref: Option<String>,
    evidence_ref: Option<String>,
}

struct RecordRule {
    kind: &'static str,
    skill: &'static str,
    required: &'static [&'static str],
}

static RECORD_RULES: &[RecordRule] = &[
    RecordRule {
        kind: "api-enumeration-observation",
        skill: ENUMERATION_SKILL,
        required: &[
            "skillId",
            "subjectRef",
            "routeRef",
            "requestRef",
            "responseRef",
            "authorizationRef",
            "ratePolicyRef",
            "observationRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "property-authorization-case",
        skill: BOPLA_SKILL,
        required: &[
            "skillId",
            "subjectRef",
            "routeRef",
            "schemaRef",
            "propertyRef",
            "identityRef",
            "roleRef",
            "responseRef",
            "authorizationRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "shadow-api-inventory",
        skill: SHADOW_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "versionRef",
            "ownerRef",
            "schemaRef",
            "documentationRef",
            "observationRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-injection-safety-case",
        skill: INJECTION_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "requestRef",
            "schemaRef",
            "parameterRef",
            "parserRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "function-authorization-plan",
        skill: BFLA_SKILL,
        required: &[
            "skillId",
            "subjectRef",
            "routeRef",
            "functionRef",
            "identityRef",
            "roleRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
];

/// Decode and validate a supplied CP09 API-security manifest without external effects.
pub(crate) fn is_valid(source: &str) -> bool {
    serde_json::from_str::<ManifestWire>(source)
        .map(|manifest| valid_manifest(&manifest))
        .unwrap_or(false)
}

fn valid_manifest(manifest: &ManifestWire) -> bool {
    [
        manifest.schema_version == 1,
        valid_texts(&[&manifest.bundle_id, &manifest.owner, &manifest.scope]),
        !manifest.evidence.is_empty(),
        manifest.evidence.iter().all(valid_evidence),
        !manifest.records.is_empty(),
        valid_records(&manifest.records),
    ]
    .into_iter()
    .all(|valid| valid)
}

fn valid_evidence(evidence: &EvidenceWire) -> bool {
    valid_texts(&[&evidence.kind, &evidence.reference])
}

fn valid_records(records: &[RecordWire]) -> bool {
    let mut seen = BTreeSet::new();
    records.iter().all(|record| {
        [seen.insert(record_skill_id(record)), valid_record(record)]
            .into_iter()
            .all(|valid| valid)
    })
}

fn valid_record(record: &RecordWire) -> bool {
    RECORD_RULES
        .iter()
        .find(|rule| rule.kind == record.kind)
        .map(|rule| {
            let encoded = serde_json::to_value(record).ok();
            [
                record.skill_id.as_deref() == Some(rule.skill),
                encoded
                    .as_ref()
                    .is_some_and(|value| valid_required_fields(value, rule.required)),
            ]
            .into_iter()
            .all(|valid| valid)
        })
        .unwrap_or(false)
}

fn valid_texts(values: &[&str]) -> bool {
    values.iter().all(|value| !value.trim().is_empty())
}

fn record_skill_id(record: &RecordWire) -> &str {
    record.skill_id.as_deref().unwrap_or("")
}

fn valid_required_fields(value: &Value, required: &[&str]) -> bool {
    required
        .iter()
        .all(|name| value.get(*name).is_some_and(valid_json_field))
}

fn valid_json_field(value: &Value) -> bool {
    value.as_str().is_some_and(|text| !text.trim().is_empty())
        || value.as_u64().is_some_and(|number| number > 0)
}
