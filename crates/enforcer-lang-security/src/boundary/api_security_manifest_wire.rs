//! Typed wire boundary for the CP09 API-security manifest packets.
//!
//! BOUNDARY-INVARIANT: this module decodes only supplied static JSON records;
//! it never calls an endpoint, browser, scanner, fuzzer, identity provider, or
//! production API.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_api_security_manifest.rs
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_api_security_manifest_b02.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_api_security_manifest.rs
// ROUNDTRIP-TEST: crates/enforcer-lang-security/tests/cyberskills_api_security_manifest_b02.rs

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const ENUMERATION_SKILL: &str = "detecting-api-enumeration-attacks";
const BOPLA_SKILL: &str = "detecting-broken-object-property-level-authorization";
const SHADOW_SKILL: &str = "detecting-shadow-api-endpoints";
const INJECTION_SKILL: &str = "exploiting-api-injection-vulnerabilities";
const BFLA_SKILL: &str = "exploiting-broken-function-level-authorization";
const EXPOSURE_SKILL: &str = "exploiting-excessive-data-exposure-in-api";
const JWT_CONFUSION_SKILL: &str = "exploiting-jwt-algorithm-confusion-attack";
const RATE_LIMIT_SKILL: &str = "implementing-api-abuse-detection-with-rate-limiting";
const GATEWAY_SKILL: &str = "implementing-api-gateway-security-controls";
const API_KEY_SKILL: &str = "implementing-api-key-security-controls";
const THROTTLING_SKILL: &str = "implementing-api-rate-limiting-and-throttling";
const SCHEMA_VALIDATION_SKILL: &str = "implementing-api-schema-validation-security";
const POSTURE_SKILL: &str = "implementing-api-security-posture-management";
const CRUNCH_SKILL: &str = "implementing-api-security-testing-with-42crunch";
const APIGEE_SKILL: &str = "implementing-api-threat-protection-with-apigee";
const RESTLER_SKILL: &str = "performing-api-fuzzing-with-restler";
const INVENTORY_DISCOVERY_SKILL: &str = "performing-api-inventory-and-discovery";
const RATE_LIMIT_BYPASS_SKILL: &str = "performing-api-rate-limiting-bypass";
const POSTMAN_SKILL: &str = "performing-api-security-testing-with-postman";
const GRAPHQL_DEPTH_SKILL: &str = "performing-graphql-depth-limit-attack";
const GRAPHQL_INTROSPECTION_SKILL: &str = "performing-graphql-introspection-attack";
const JWT_NONE_SKILL: &str = "performing-jwt-none-algorithm-attack";
const SOAP_SECURITY_SKILL: &str = "performing-soap-web-service-security-testing";
const API_AUTHENTICATION_SKILL: &str = "testing-api-authentication-weaknesses";
const BOLA_SKILL: &str = "testing-api-for-broken-object-level-authorization";
const MASS_ASSIGNMENT_SKILL: &str = "testing-api-for-mass-assignment-vulnerability";
const OAUTH_SKILL: &str = "testing-oauth2-implementation-flaws";
const WEBSOCKET_SKILL: &str = "testing-websocket-api-security";

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
    field_ref: Option<String>,
    sensitivity_ref: Option<String>,
    comparison_ref: Option<String>,
    token_ref: Option<String>,
    header_ref: Option<String>,
    algorithm_ref: Option<String>,
    key_ref: Option<String>,
    verification_ref: Option<String>,
    client_ref: Option<String>,
    policy_ref: Option<String>,
    capacity_ref: Option<String>,
    window_ref: Option<String>,
    threshold_ref: Option<String>,
    telemetry_ref: Option<String>,
    gateway_ref: Option<String>,
    auth_policy_ref: Option<String>,
    tls_policy_ref: Option<String>,
    waf_policy_ref: Option<String>,
    backend_ref: Option<String>,
    key_policy_ref: Option<String>,
    generation_ref: Option<String>,
    storage_ref: Option<String>,
    hashing_ref: Option<String>,
    rotation_ref: Option<String>,
    revocation_ref: Option<String>,
    leak_monitoring_ref: Option<String>,
    inventory_ref: Option<String>,
    finding_ref: Option<String>,
    score_ref: Option<String>,
    exception_ref: Option<String>,
    remediation_ref: Option<String>,
    definition_ref: Option<String>,
    audit_ref: Option<String>,
    report_ref: Option<String>,
    json_policy_ref: Option<String>,
    xml_policy_ref: Option<String>,
    regex_policy_ref: Option<String>,
    spike_arrest_ref: Option<String>,
    oauth_policy_ref: Option<String>,
    api_key_policy_ref: Option<String>,
    openapi_ref: Option<String>,
    grammar_ref: Option<String>,
    dictionary_ref: Option<String>,
    sequence_ref: Option<String>,
    checker_ref: Option<String>,
    auth_config_ref: Option<String>,
    fuzz_budget_ref: Option<String>,
    bug_report_ref: Option<String>,
    traffic_ref: Option<String>,
    dns_ref: Option<String>,
    endpoint_ref: Option<String>,
    shadow_ref: Option<String>,
    cloud_inventory_ref: Option<String>,
    js_analysis_ref: Option<String>,
    baseline_ref: Option<String>,
    variation_ref: Option<String>,
    method_ref: Option<String>,
    content_type_ref: Option<String>,
    volume_ref: Option<String>,
    collection_ref: Option<String>,
    environment_ref: Option<String>,
    test_ref: Option<String>,
    proxy_ref: Option<String>,
    query_ref: Option<String>,
    depth_ref: Option<String>,
    alias_ref: Option<String>,
    complexity_ref: Option<String>,
    batch_ref: Option<String>,
    introspection_ref: Option<String>,
    visibility_ref: Option<String>,
    query_policy_ref: Option<String>,
    field_policy_ref: Option<String>,
    signature_ref: Option<String>,
    claims_ref: Option<String>,
    wsdl_ref: Option<String>,
    operation_ref: Option<String>,
    xml_parser_ref: Option<String>,
    xxe_policy_ref: Option<String>,
    xpath_policy_ref: Option<String>,
    soap_action_ref: Option<String>,
    ws_security_ref: Option<String>,
    auth_scheme_ref: Option<String>,
    token_validation_ref: Option<String>,
    credential_policy_ref: Option<String>,
    session_policy_ref: Option<String>,
    coverage_ref: Option<String>,
    object_ref: Option<String>,
    tenant_ref: Option<String>,
    matrix_ref: Option<String>,
    variant_ref: Option<String>,
    field_allowlist_ref: Option<String>,
    bindable_ref: Option<String>,
    input_ref: Option<String>,
    ownership_ref: Option<String>,
    flow_ref: Option<String>,
    redirect_ref: Option<String>,
    scope_policy_ref: Option<String>,
    state_ref: Option<String>,
    pkce_ref: Option<String>,
    issuer_ref: Option<String>,
    token_policy_ref: Option<String>,
    authorization_endpoint_ref: Option<String>,
    origin_ref: Option<String>,
    handshake_ref: Option<String>,
    subprotocol_ref: Option<String>,
    message_ref: Option<String>,
    session_ref: Option<String>,
    channel_ref: Option<String>,
    frame_ref: Option<String>,
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
    RecordRule {
        kind: "excessive-data-exposure-case",
        skill: EXPOSURE_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "responseRef",
            "schemaRef",
            "fieldRef",
            "sensitivityRef",
            "comparisonRef",
            "authorizationRef",
            "scopeRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "jwt-algorithm-confusion-case",
        skill: JWT_CONFUSION_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "tokenRef",
            "headerRef",
            "algorithmRef",
            "keyRef",
            "verificationRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "rate-limit-control-plan",
        skill: RATE_LIMIT_SKILL,
        required: &[
            "skillId",
            "clientRef",
            "policyRef",
            "algorithmRef",
            "capacityRef",
            "windowRef",
            "thresholdRef",
            "telemetryRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-gateway-control-plan",
        skill: GATEWAY_SKILL,
        required: &[
            "skillId",
            "gatewayRef",
            "authPolicyRef",
            "ratePolicyRef",
            "schemaRef",
            "tlsPolicyRef",
            "wafPolicyRef",
            "backendRef",
            "scopeRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-key-control-plan",
        skill: API_KEY_SKILL,
        required: &[
            "skillId",
            "keyPolicyRef",
            "generationRef",
            "storageRef",
            "hashingRef",
            "scopeRef",
            "ratePolicyRef",
            "rotationRef",
            "revocationRef",
            "leakMonitoringRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-rate-limiting-throttling-plan",
        skill: THROTTLING_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "policyRef",
            "algorithmRef",
            "clientRef",
            "capacityRef",
            "windowRef",
            "backendRef",
            "responseRef",
            "telemetryRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-schema-validation-security-case",
        skill: SCHEMA_VALIDATION_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "schemaRef",
            "requestRef",
            "responseRef",
            "parserRef",
            "parameterRef",
            "authorizationRef",
            "scopeRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-security-posture-review",
        skill: POSTURE_SKILL,
        required: &[
            "skillId",
            "inventoryRef",
            "findingRef",
            "scoreRef",
            "exceptionRef",
            "remediationRef",
            "ownerRef",
            "scopeRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-42crunch-contract-audit",
        skill: CRUNCH_SKILL,
        required: &[
            "skillId",
            "definitionRef",
            "schemaRef",
            "auditRef",
            "scoreRef",
            "findingRef",
            "reportRef",
            "scopeRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "apigee-threat-protection-plan",
        skill: APIGEE_SKILL,
        required: &[
            "skillId",
            "gatewayRef",
            "jsonPolicyRef",
            "xmlPolicyRef",
            "regexPolicyRef",
            "spikeArrestRef",
            "oauthPolicyRef",
            "apiKeyPolicyRef",
            "scopeRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-fuzzing-restler-plan",
        skill: RESTLER_SKILL,
        required: &[
            "skillId",
            "openapiRef",
            "grammarRef",
            "dictionaryRef",
            "sequenceRef",
            "checkerRef",
            "authConfigRef",
            "fuzzBudgetRef",
            "bugReportRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-inventory-discovery-plan",
        skill: INVENTORY_DISCOVERY_SKILL,
        required: &[
            "skillId",
            "inventoryRef",
            "trafficRef",
            "dnsRef",
            "endpointRef",
            "shadowRef",
            "cloudInventoryRef",
            "jsAnalysisRef",
            "scopeRef",
            "authorizationRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-rate-limit-bypass-case",
        skill: RATE_LIMIT_BYPASS_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "ratePolicyRef",
            "baselineRef",
            "headerRef",
            "variationRef",
            "methodRef",
            "contentTypeRef",
            "volumeRef",
            "observationRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-postman-security-test-plan",
        skill: POSTMAN_SKILL,
        required: &[
            "skillId",
            "collectionRef",
            "environmentRef",
            "requestRef",
            "roleRef",
            "authPolicyRef",
            "testRef",
            "proxyRef",
            "reportRef",
            "scopeRef",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "graphql-depth-limit-assessment",
        skill: GRAPHQL_DEPTH_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "schemaRef",
            "queryRef",
            "depthRef",
            "aliasRef",
            "complexityRef",
            "batchRef",
            "responseRef",
            "observationRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "graphql-introspection-assessment",
        skill: GRAPHQL_INTROSPECTION_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "schemaRef",
            "introspectionRef",
            "visibilityRef",
            "queryPolicyRef",
            "fieldPolicyRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "jwt-none-algorithm-assessment",
        skill: JWT_NONE_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "tokenRef",
            "headerRef",
            "algorithmRef",
            "signatureRef",
            "keyRef",
            "claimsRef",
            "verificationRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "soap-wsdl-security-assessment",
        skill: SOAP_SECURITY_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "wsdlRef",
            "operationRef",
            "xmlParserRef",
            "xxePolicyRef",
            "xpathPolicyRef",
            "soapActionRef",
            "wsSecurityRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "api-authentication-control-assessment",
        skill: API_AUTHENTICATION_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "authSchemeRef",
            "authPolicyRef",
            "tokenValidationRef",
            "identityRef",
            "credentialPolicyRef",
            "sessionPolicyRef",
            "coverageRef",
            "rotationRef",
            "revocationRef",
            "leakMonitoringRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "object-authorization-case",
        skill: BOLA_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "schemaRef",
            "objectRef",
            "tenantRef",
            "identityRef",
            "roleRef",
            "matrixRef",
            "variantRef",
            "requestRef",
            "responseRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "mass-assignment-control-assessment",
        skill: MASS_ASSIGNMENT_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "schemaRef",
            "requestRef",
            "inputRef",
            "fieldAllowlistRef",
            "bindableRef",
            "ownershipRef",
            "roleRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "oauth2-flow-security-assessment",
        skill: OAUTH_SKILL,
        required: &[
            "skillId",
            "flowRef",
            "clientRef",
            "authorizationEndpointRef",
            "redirectRef",
            "scopePolicyRef",
            "stateRef",
            "pkceRef",
            "issuerRef",
            "tokenPolicyRef",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "websocket-api-security-assessment",
        skill: WEBSOCKET_SKILL,
        required: &[
            "skillId",
            "routeRef",
            "originRef",
            "handshakeRef",
            "subprotocolRef",
            "messageRef",
            "sessionRef",
            "channelRef",
            "frameRef",
            "authPolicyRef",
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
