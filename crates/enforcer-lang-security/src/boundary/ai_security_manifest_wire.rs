//! Typed wire deserialization boundary for the CP09 AI-security manifest packets.
//!
//! BOUNDARY-INVARIANT: only this module decodes untrusted JSON into private
//! wire shapes. The rule layer receives a boolean validation result and never
//! invokes a model, server, external engine, or production endpoint.
// NEGATIVE-TEST: crates/enforcer-lang-security/tests/cyberskills_ai_security_manifest.rs

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const VECTOR_SKILL: &str = "assessing-vector-and-embedding-weaknesses";
const MCP_SKILL: &str = "auditing-mcp-servers-for-tool-poisoning";
const RED_TEAM_SKILL: &str = "continuous-llm-red-teaming-with-promptfoo";
const GUARDRAIL_SKILL: &str = "defending-llms-with-guardrails";
const PROMPT_INJECTION_SKILL: &str = "detecting-ai-model-prompt-injection-attacks";
const POISONING_SKILL: &str = "detecting-data-and-model-poisoning";
const INDIRECT_INJECTION_SKILL: &str = "detecting-indirect-prompt-injection";
const EXTRACTION_SKILL: &str = "detecting-model-extraction-attacks";
const SECURITY_GUARDRAIL_SKILL: &str = "implementing-llm-guardrails-for-security";
const PYRIT_SKILL: &str = "orchestrating-llm-attacks-with-pyrit";
const GARAK_SKILL: &str = "red-teaming-llms-with-garak";
const AGENT_TOOL_SKILL: &str = "securing-agentic-ai-tool-invocation";
const SYSTEM_PROMPT_SKILL: &str = "testing-for-system-prompt-leakage";
const RAG_INJECTION_SKILL: &str = "testing-prompt-injection-in-rag-pipelines";

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

// BRAND-INVARIANT: evidence text is accepted only when both role fields are
// non-empty in `valid_evidence`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceWire {
    kind: String,
    reference: String,
}

// BRAND-INVARIANT: each record is a typed serialized shape whose required
// fields are selected by the closed kind-to-skill predicate table below.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecordWire {
    kind: String,
    skill_id: Option<String>,
    subject_id: Option<String>,
    model_id: Option<String>,
    embedding_dimension: Option<u32>,
    index_id: Option<String>,
    query_id: Option<String>,
    document_id: Option<String>,
    tenant_id: Option<String>,
    policy_id: Option<String>,
    server_id: Option<String>,
    tool_name: Option<String>,
    schema_ref: Option<String>,
    prompt_ref: Option<String>,
    instruction_ref: Option<String>,
    permission_set: Option<String>,
    transport: Option<String>,
    scenario_id: Option<String>,
    prompt_id: Option<String>,
    expected_outcome: Option<String>,
    evidence_ref: Option<String>,
    input_class: Option<String>,
    action: Option<String>,
    escalation: Option<String>,
    audit_ref: Option<String>,
    source: Option<String>,
    trust_boundary: Option<String>,
    classification: Option<String>,
    response_ref: Option<String>,
    provenance_ref: Option<String>,
    integrity_ref: Option<String>,
    sanitization_ref: Option<String>,
    telemetry_ref: Option<String>,
    output_class: Option<String>,
    authorization_ref: Option<String>,
    scope_ref: Option<String>,
    stop_condition: Option<String>,
    review_ref: Option<String>,
    disclosure_ref: Option<String>,
    retrieval_ref: Option<String>,
}

struct RecordRule {
    kind: &'static str,
    skill: &'static str,
    required: &'static [&'static str],
}

static RECORD_RULES: &[RecordRule] = &[
    RecordRule {
        kind: "vector-evaluation",
        skill: VECTOR_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "modelId",
            "embeddingDimension",
            "indexId",
            "queryId",
            "documentId",
            "tenantId",
            "policyId",
        ],
    },
    RecordRule {
        kind: "mcp-tool-definition",
        skill: MCP_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "serverId",
            "toolName",
            "schemaRef",
            "promptRef",
            "instructionRef",
            "permissionSet",
            "transport",
        ],
    },
    RecordRule {
        kind: "red-team-scenario",
        skill: RED_TEAM_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "scenarioId",
            "promptId",
            "expectedOutcome",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "guardrail-policy",
        skill: GUARDRAIL_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "policyId",
            "inputClass",
            "action",
            "escalation",
            "auditRef",
        ],
    },
    RecordRule {
        kind: "prompt-event",
        skill: PROMPT_INJECTION_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "source",
            "trustBoundary",
            "classification",
            "responseRef",
        ],
    },
    RecordRule {
        kind: "model-integrity-evidence",
        skill: POISONING_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "modelId",
            "source",
            "classification",
            "provenanceRef",
            "integrityRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "indirect-content-observation",
        skill: INDIRECT_INJECTION_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "source",
            "trustBoundary",
            "classification",
            "sanitizationRef",
            "responseRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "inference-audit-event",
        skill: EXTRACTION_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "modelId",
            "queryId",
            "source",
            "classification",
            "action",
            "telemetryRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "guardrail-evaluation",
        skill: SECURITY_GUARDRAIL_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "policyId",
            "inputClass",
            "outputClass",
            "action",
            "escalation",
            "auditRef",
        ],
    },
    RecordRule {
        kind: "authorized-ai-evaluation-plan",
        skill: PYRIT_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "scenarioId",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "authorized-red-team-plan",
        skill: GARAK_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "scenarioId",
            "authorizationRef",
            "scopeRef",
            "stopCondition",
            "reviewRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "agent-tool-policy",
        skill: AGENT_TOOL_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "serverId",
            "toolName",
            "schemaRef",
            "permissionSet",
            "authorizationRef",
            "scopeRef",
            "action",
            "auditRef",
        ],
    },
    RecordRule {
        kind: "prompt-confidentiality-case",
        skill: SYSTEM_PROMPT_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "modelId",
            "promptRef",
            "instructionRef",
            "classification",
            "disclosureRef",
            "responseRef",
            "evidenceRef",
        ],
    },
    RecordRule {
        kind: "rag-injection-observation",
        skill: RAG_INJECTION_SKILL,
        required: &[
            "skillId",
            "subjectId",
            "indexId",
            "documentId",
            "promptRef",
            "source",
            "trustBoundary",
            "classification",
            "retrievalRef",
            "responseRef",
            "evidenceRef",
        ],
    },
];

/// Decode and validate a supplied CP09 B01 manifest without external effects.
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
