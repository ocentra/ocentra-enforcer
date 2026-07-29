//! Event schema + wire-hash canonicalization.
//!
//! Ported from `src/coordination/vendor/events.js`. The wire-hash allow-set,
//! canonicalization ordering, and the golden compatibility sentinel are
//! copied EXACTLY: a Rust port that diverges here makes every existing
//! `.mjs`-produced ledger fail hash checks and breaks cross-impl sync
//! (arc-16 workpack, "Wire-hash vs extension-hash canonicalization" row).

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::error::{CoordinationError, Result};
use enforcer_domain::coordination_types::{ClaimEventId, CoordinationRejection};

pub mod boundary;

use boundary::HubEventResponse;

// The coordination event envelope moved to `events::boundary`.
// The prior raw DTO documentation remains represented by that boundary type.
// Field values here are stored as plain `String`/`Value`; the boundary owns them.
// the domain-shape brand-checking at the API boundary (arc-16 `api.rs`),
// not inside every event read, matching the workpack's own note that
// context is opaque/`Schema.Unknown` in the JS source.
/* event DTO moved to events::boundary
/// Caller/environment context (`machine`, `worktreeRoot`, `branch`,
/// `commit`, `codexThreadId`, ...). EXCLUDED from the wire hash — see
/// `wire_hash_event`. L2 finding: the api layer must populate this from
/// CALLER-supplied identity params, never the server's own cwd.
*/

/// The exact set of fields that participate in the wire hash. Copied
/// verbatim from `events.js#WIRE_HASH_FIELDS` (`context` is deliberately
/// excluded).
const WIRE_HASH_FIELDS: &[&str] = &[
    "id",
    "schema",
    "hub",
    "nodeId",
    "nodeName",
    "lane",
    "writer",
    "type",
    "ts",
    "seq",
    "prevEventId",
    "prevHash",
    "to",
    "body",
    "messageId",
    "paths",
    "reason",
    "owner",
    "owners",
    "state",
    "workerState",
    "taskId",
    "taskState",
    "title",
    "prUrl",
    "summary",
    "ttlSeconds",
    "sessionId",
];

/// Golden sentinel: hashing `HASH_COMPATIBILITY_SAMPLE_EVENT` (below) MUST
/// equal this value. Copied verbatim from `events.js`.
pub const EXPECTED_HASH_COMPATIBILITY_WIRE_HASH: &str =
    "sha256:c4333184613bd63a0d2918e3be4c88ce2ea4a32d9fc7e07bb755ade688aada76";

/// Build the sample event used by the golden compatibility check. Field
/// values are copied verbatim from `events.js#HASH_COMPATIBILITY_SAMPLE_EVENT`.
fn hash_compatibility_sample_event() -> Value {
    serde_json::json!({
        "id": "evt_compatibility0000000000000000000000",
        "schema": 1,
        "hub": "compat-hub",
        "nodeId": "node_compatibility",
        "nodeName": "CompatNode",
        "lane": "codex-a",
        "writer": "node_compatibility.codex-a",
        "type": "claim",
        "ts": "2026-06-30T00:00:00.000Z",
        "seq": 1,
        "prevEventId": null,
        "prevHash": null,
        "paths": ["src/lib.rs"],
        "reason": "compatibility sentinel",
        "context": {
            "projectId": "must-not-affect-wire-hash",
            "repoRoot": "C:/repo",
        },
    })
}

/// Compute the wire hash of an arbitrary JSON event value (any shape — used
/// both for the golden fixture and for real completed events serialized as
/// `Value`). Mirrors `events.js#hashForEvent`.
pub fn hash_for_event_value(event: &Value) -> String {
    let wire = wire_hash_event(event);
    let canonical = canonicalize(&wire);
    let serialized = serde_json::to_string(&canonical).unwrap_or_default();
    format!("sha256:{:x}", Sha256::digest(serialized.as_bytes()))
}

/// Compute the hash INCLUDING excluded fields like `context` (used only to
/// prove the golden sentinel changes when context is folded in — i.e. proves
/// `context` really is excluded from the real wire hash).
pub fn hash_for_event_with_extensions_value(event: &Value) -> String {
    let canonical = canonicalize(event);
    let serialized = serde_json::to_string(&canonical).unwrap_or_default();
    format!("sha256:{:x}", Sha256::digest(serialized.as_bytes()))
}

/// Result of `coordination_hash_compatibility` — mirrors
/// `events.js#coordinationHashCompatibility`.
#[derive(Debug, Clone, PartialEq)]
/// Result of comparing Rust event hashing with the frozen compatibility sentinel.
pub struct HashCompatibility {
    pub ok: bool,
    pub expected_wire_hash: String,
    pub actual_wire_hash: String,
    pub extension_hash: String,
    pub context_excluded_from_wire_hash: bool,
}

/// Run the golden compatibility check against the fixed sample event.
pub fn coordination_hash_compatibility() -> HashCompatibility {
    let sample = hash_compatibility_sample_event();
    let actual_wire_hash = hash_for_event_value(&sample);
    let extension_hash = hash_for_event_with_extensions_value(&sample);
    let ok = actual_wire_hash == EXPECTED_HASH_COMPATIBILITY_WIRE_HASH
        && extension_hash != EXPECTED_HASH_COMPATIBILITY_WIRE_HASH;
    HashCompatibility {
        ok,
        expected_wire_hash: EXPECTED_HASH_COMPATIBILITY_WIRE_HASH.to_owned(),
        actual_wire_hash,
        extension_hash: extension_hash.clone(),
        context_excluded_from_wire_hash: extension_hash != hash_for_event_value(&sample),
    }
}

/// Assert the golden compatibility check passes, or return an error. Mirrors
/// `events.js#assertCoordinationHashCompatibility`.
pub fn assert_coordination_hash_compatibility() -> Result<()> {
    let result = coordination_hash_compatibility();
    if result.ok {
        Ok(())
    } else {
        let reason = CoordinationRejection::parse(&format!(
            "coordination hash compatibility check failed: expected {}, got {}",
            result.expected_wire_hash, result.actual_wire_hash
        ))?;
        Err(CoordinationError::rejected(reason))
    }
}

/// Compute the wire hash of a real `HubEvent` (excluding its own `hash`
/// field, matching `events.js#hashForEvent(withoutHash(event))`).
pub fn hash_for_event(event: &HubEventResponse) -> Result<String> {
    let mut value = serde_json::to_value(event)?;
    if let Value::Object(map) = &mut value {
        map.remove("hash");
    }
    Ok(hash_for_event_value(&value))
}

/// Verify a stored event's `hash` field matches its recomputed wire hash.
/// Mirrors `events.js#assertEventHash`.
pub fn assert_event_hash(event: &HubEventResponse) -> Result<()> {
    let expected = hash_for_event(event)?;
    if event.hash == expected {
        Ok(())
    } else {
        Err(CoordinationError::HashMismatch {
            event_id: ClaimEventId::try_from(event.id.clone())?,
        })
    }
}

/// Filter a JSON event object down to only the wire-hash-participating
/// fields (dropping `null`/absent values), mirroring
/// `events.js#wireHashEvent`. Note: the JS source drops `undefined` but
/// KEEPS `null` (e.g. `prevEventId: null`) — this must match exactly for the
/// golden sentinel to reproduce.
fn wire_hash_event(event: &Value) -> Value {
    let Value::Object(map) = event else {
        return event.clone();
    };
    let mut filtered = Map::new();
    for key in WIRE_HASH_FIELDS {
        if let Some(v) = map.get(*key) {
            filtered.insert((*key).to_owned(), v.clone());
        }
    }
    Value::Object(filtered)
}

/// Recursively sort object keys and drop `null`... — NO: the JS
/// `canonicalize` only drops `undefined` (which doesn't exist as a distinct
/// JSON value), so `null` values are preserved; only key ORDER changes.
/// Arrays are canonicalized element-wise, preserving order.
fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map
                .iter()
                .map(|(k, v)| (k.clone(), canonicalize(v)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut sorted = Map::new();
            for (k, v) in entries {
                sorted.insert(k, v);
            }
            Value::Object(sorted)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::boundary::HubEventResponse;
    use super::{
        assert_event_hash, coordination_hash_compatibility, hash_compatibility_sample_event,
        hash_for_event, hash_for_event_value, hash_for_event_with_extensions_value,
        EXPECTED_HASH_COMPATIBILITY_WIRE_HASH,
    };
    use crate::error::CoordinationError;
    use serde_json::Value;

    #[test]
    fn golden_sentinel_matches_expected_wire_hash() {
        let result = coordination_hash_compatibility();
        assert_eq!(
            result.actual_wire_hash,
            EXPECTED_HASH_COMPATIBILITY_WIRE_HASH
        );
        assert!(result.ok, "golden fixture must pass: {result:?}");
    }

    #[test]
    fn mutating_context_does_not_change_wire_hash() {
        let mut sample = hash_compatibility_sample_event();
        let baseline = hash_for_event_value(&sample);
        sample["context"]["projectId"] = Value::String("something-else-entirely".into());
        let mutated = hash_for_event_value(&sample);
        assert_eq!(
            baseline, mutated,
            "context mutation must not affect wire hash"
        );
    }

    #[test]
    fn mutating_a_wire_field_changes_the_hash() {
        let mut sample = hash_compatibility_sample_event();
        let baseline = hash_for_event_value(&sample);
        sample["reason"] = Value::String("different reason".into());
        let mutated = hash_for_event_value(&sample);
        assert_ne!(
            baseline, mutated,
            "wire-field mutation must change the hash"
        );
    }

    #[test]
    fn extension_hash_differs_from_wire_hash_because_context_is_included() {
        let sample = hash_compatibility_sample_event();
        let wire = hash_for_event_value(&sample);
        let extended = hash_for_event_with_extensions_value(&sample);
        assert_ne!(wire, extended);
    }

    #[test]
    fn assert_event_hash_detects_tampering() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let event = HubEventResponse {
            id: "evt_test".into(),
            schema: 1,
            hub: "hub".into(),
            node_id: "node_x".into(),
            node_name: "Node".into(),
            lane: "arc-16".into(),
            writer: "node_x.arc-16".into(),
            kind: "claim".into(),
            ts: "2026-07-04T00:00:00.000Z".into(),
            seq: 1,
            prev_event_id: None,
            prev_hash: None,
            hash: String::new(),
            to: None,
            body: None,
            message_id: None,
            paths: Some(vec!["src/lib.rs".into()]),
            reason: None,
            owner: None,
            owners: None,
            state: None,
            worker_state: None,
            task_id: None,
            task_state: None,
            title: None,
            pr_url: None,
            summary: None,
            ttl_seconds: None,
            session_id: None,
            context: None,
        };
        let mut completed = event.clone();
        completed.hash = hash_for_event(&event)?;
        assert_event_hash(&completed)?;

        let expected_event_id = completed.id.clone();
        let mut tampered = completed;
        tampered.reason = Some("tampered".into());
        match assert_event_hash(&tampered) {
            Err(CoordinationError::HashMismatch { event_id }) => {
                assert_eq!(event_id.as_str(), expected_event_id);
            }
            other => {
                return Err(
                    format!("expected hash mismatch for tampered event, got {other:?}").into(),
                )
            }
        }
        Ok(())
    }

    #[test]
    fn legacy_camel_case_stream_event_decodes_and_retains_its_wire_hash(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let raw = serde_json::json!({
            "id": "evt_099919a29a5e422b839b850265207b1b",
            "schema": 1,
            "hub": "ocentra-enforcer",
            "nodeId": "node_7450523d7490414f86992de67525c1c2",
            "nodeName": "GameDev",
            "lane": "codex-a",
            "writer": "node_7450523d7490414f86992de67525c1c2.codex-a",
            "type": "claim",
            "ts": "2026-07-01T06:42:40.372Z",
            "seq": 1,
            "prevEventId": null,
            "prevHash": null,
            "hash": "sha256:33b3162802c03d2e780d429d12e2711e80d91beeff2c3db70f8d9c3c53c10602",
            "paths": ["scripts/test/portal-e2e-runner.test.mjs"],
            "reason": "proof migration fix portal tooling profile reference",
            "context": {
                "projectId": "ocentra-OcentraParent",
                "branch": "codex/tracking-plan-full-continuation-a"
            }
        });

        let event: HubEventResponse = serde_json::from_value(raw)?;
        let rendered = serde_json::to_value(&event)?;

        assert_eq!(
            rendered.get("nodeId").and_then(Value::as_str),
            Some("node_7450523d7490414f86992de67525c1c2")
        );
        assert_eq!(rendered.get("node_id"), None);
        assert_event_hash(&event)?;
        Ok(())
    }
}
