//! Derived ledger-view boundary. Raw event and JSON view fields stay here;
//! canonical append-only streams remain the only coordination authority.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::events::boundary::HubEventResponse;
use crate::lock::RawClaim;

// SERIALIZATION-DOC: these read-only DTOs render a replay of hash-verified
// stream events; no writer consumes them as an authoritative state source.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerSnapshot {
    pub events: Vec<HubEventResponse>,
    pub active_claims: Vec<RawClaim>,
    pub reports: Vec<HubEventResponse>,
    pub workers: Vec<HubEventResponse>,
    pub tasks: Vec<HubEventResponse>,
    pub inbox: BTreeMap<String, Vec<InboxItemDto>>,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct InboxItemDto {
    pub event: HubEventResponse,
    pub acknowledged_by: Vec<String>,
}

pub fn materialize(root: &Path) -> crate::error::Result<LedgerSnapshot> {
    let all = crate::sync::stream::read_all_streams(root)?;
    let events = all.events;
    let digest = snapshot_digest(&events)?;
    let active_claims = super::active_claims(&events);
    let reports = events
        .iter()
        .filter(|event| event.kind == "report")
        .cloned()
        .collect();
    let workers = events
        .iter()
        .filter(|event| {
            matches!(
                event.kind.as_str(),
                "worker.update" | "status" | "heartbeat" | "report"
            )
        })
        .cloned()
        .collect();
    let tasks = events
        .iter()
        .filter(|event| event.kind == "task.update")
        .cloned()
        .collect();
    let mut acknowledged: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for event in &events {
        if event.kind == "ack" {
            if let Some(message_id) = &event.message_id {
                acknowledged
                    .entry(message_id.clone())
                    .or_default()
                    .insert(event.writer.clone());
            }
        }
    }
    let mut inbox = BTreeMap::new();
    for event in &events {
        if matches!(event.kind.as_str(), "message" | "handoff") {
            if let Some(target) = &event.to {
                inbox
                    .entry(target.clone())
                    .or_insert_with(Vec::new)
                    .push(InboxItemDto {
                        event: event.clone(),
                        acknowledged_by: acknowledged
                            .get(&event.id)
                            .map_or_else(Vec::new, |writers| writers.iter().cloned().collect()),
                    });
            }
        }
    }
    Ok(LedgerSnapshot {
        events,
        active_claims,
        reports,
        workers,
        tasks,
        inbox,
        digest,
    })
}

fn snapshot_digest(events: &[HubEventResponse]) -> crate::error::Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(events)?)
    ))
}
