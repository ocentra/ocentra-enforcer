//! Serialized coordination event boundary.
//!
//! BOUNDARY-INVARIANT: raw MJS-compatible ledger fields are decoded and
//! encoded only in this wire record; coordination decisions consume validated
//! identities materialized from it.
//! BOUNDARY-TEST: malformed JSON and hash-tampered events are rejected by the
//! event and stream contract tests.
//! BOUNDARY-OWNER: enforcer-coordination.
//! boundaryOwnerNote: enforcer-coordination owns this legacy ledger wire
//! contract and its conversion into validated claim context.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// SERIALIZATION-DOC: field names and omission rules preserve the existing MJS ledger wire contract.
/// Stable JSON response for one append-only coordination event.
/// Round-trip compatibility is verified by `response_round_trip_preserves_the_wire_contract`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubEventResponse {
    pub id: String,
    pub schema: u32,
    pub hub: String,
    pub node_id: String,
    pub node_name: String,
    pub lane: String,
    pub writer: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub ts: String,
    pub seq: u64,
    pub prev_event_id: Option<String>,
    pub prev_hash: Option<String>,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owners: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
}

/// Decode one external NDJSON event line into its stable wire record.
pub fn decode_event_line(line: &str) -> crate::error::Result<HubEventResponse> {
    Ok(serde_json::from_str(line)?)
}

impl HubEventResponse {
    /// Convert one wire claim response into the fully validated claim projection.
    pub(crate) fn to_domain(&self) -> crate::error::Result<crate::lock::RawClaim> {
        use enforcer_domain::coordination_types::{ClaimEventId, ClaimLane, ClaimWriter};

        Ok(crate::lock::RawClaim {
            // CLONE-JUSTIFICATION: the persisted wire response remains borrowed while the domain claim owns identity.
            writer: ClaimWriter::parse(self.writer.clone())?,
            // CLONE-JUSTIFICATION: the persisted wire response remains borrowed while the domain claim owns identity.
            lane: ClaimLane::parse(self.lane.clone())?,
            paths: to_domain_claim_paths(self.paths.as_deref().unwrap_or_default())?,
            // CLONE-JUSTIFICATION: the persisted wire response remains borrowed while the domain claim owns identity.
            event_id: ClaimEventId::parse(self.id.clone())?,
            reason: to_domain_claim_reason(self.reason.as_deref())?,
            context: to_domain_claim_context(self)?,
        })
    }
}

fn to_domain_claim_context(
    event: &HubEventResponse,
) -> crate::error::Result<crate::lock::ClaimContext> {
    use enforcer_domain::coordination_types::{
        ClaimGroup, CoordinationBranch, CoordinationOwnerIdentity, CoordinationProjectId,
        CoordinationRepository, CoordinationWorktree, LockKind, Operation,
    };

    let Some(value) = &event.context else {
        return Ok(crate::lock::ClaimContext::default());
    };
    let get = |key: &str| value.get(key).and_then(serde_json::Value::as_str);
    Ok(crate::lock::ClaimContext {
        project_id: get("projectId")
            .map(CoordinationProjectId::parse)
            .transpose()?,
        git_remote: get("gitRemote")
            .map(CoordinationRepository::parse)
            .transpose()?,
        repo_root: get("repoRoot")
            .map(CoordinationRepository::parse)
            .transpose()?,
        worktree_root: get("worktreeRoot")
            .map(CoordinationWorktree::parse)
            .transpose()?,
        branch: get("branch").map(CoordinationBranch::parse).transpose()?,
        codex_thread_id: get("codexThreadId")
            .map(CoordinationOwnerIdentity::parse)
            .transpose()?,
        codex_session_id: get("codexSessionId")
            .map(CoordinationOwnerIdentity::parse)
            .transpose()?,
        explicit_codex_thread_id: get("explicitCodexThreadId")
            .map(CoordinationOwnerIdentity::parse)
            .transpose()?,
        explicit_codex_session_id: get("explicitCodexSessionId")
            .map(CoordinationOwnerIdentity::parse)
            .transpose()?,
        claim_group: get("claimGroup").map(ClaimGroup::parse).transpose()?,
        lock_kind: get("lockKind").map(LockKind::parse).transpose()?,
        operation: get("operation").map(Operation::parse).transpose()?,
    })
}

fn to_domain_claim_paths(
    paths: &[String],
) -> crate::error::Result<Vec<enforcer_domain::coordination_types::ClaimPath>> {
    paths
        .iter()
        .map(|path| enforcer_domain::coordination_types::ClaimPath::parse(path))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Convert valid raw claim paths while ignoring malformed legacy entries.
pub(crate) fn to_domain_valid_claim_paths(
    paths: &[String],
) -> Vec<enforcer_domain::coordination_types::ClaimPath> {
    paths
        .iter()
        .filter_map(|path| enforcer_domain::coordination_types::ClaimPath::parse(path).ok())
        .collect()
}

/// Convert valid raw claim-owner identities while ignoring malformed legacy entries.
pub(crate) fn to_domain_valid_claim_writers(
    writers: &[String],
) -> Vec<enforcer_domain::coordination_types::ClaimWriter> {
    writers
        .iter()
        // CLONE-JUSTIFICATION: each accepted domain writer owns text borrowed from the legacy wire collection.
        .filter_map(|writer| {
            enforcer_domain::coordination_types::ClaimWriter::parse(writer.clone()).ok()
        })
        .collect()
}

fn to_domain_claim_reason(
    reason: Option<&str>,
) -> crate::error::Result<Option<enforcer_domain::coordination_types::ClaimReason>> {
    reason
        .map(enforcer_domain::coordination_types::ClaimReason::parse)
        .transpose()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::HubEventResponse;
    use crate::error::Result;

    #[test]
    fn response_round_trip_preserves_the_wire_contract() -> Result<()> {
        let json = serde_json::json!({
            "id": "evt_boundary_round_trip",
            "schema": 1,
            "hub": "test-hub",
            "nodeId": "node_test",
            "nodeName": "TestNode",
            "lane": "codex-test",
            "writer": "node_test.codex-test",
            "type": "claim",
            "ts": "2026-07-16T00:00:00.000Z",
            "seq": 1,
            "prevEventId": null,
            "prevHash": null,
            "hash": "sha256:test"
        });
        let response: HubEventResponse = serde_json::from_value(json.clone())?;
        assert_eq!(serde_json::to_value(response)?, json);
        Ok(())
    }
}
