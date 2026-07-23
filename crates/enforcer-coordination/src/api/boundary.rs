//! Filesystem and JSON transport boundary for the coordination command API.
//!
//! BOUNDARY-INVARIANT: raw serialized identity/event fields are materialized
//! only here and converted to canonical domain values before command logic.
//! boundaryOwnerNote: enforcer-coordination owns this command transport boundary.

use enforcer_domain::coordination_types::{
    ClaimEventId, ClaimPath, ClaimReason, CoordinationEventKind, CoordinationMessageBody,
    CoordinationRepoRoot, CoordinationTimestamp, NodeId, NodeName, WriterId,
};
use enforcer_domain::ids::LaneId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

use super::{CallerContext, Hub};
use crate::domain::HubConfig;
use crate::error::{CoordinationError, Result};
use crate::events::boundary::HubEventResponse;
use crate::lock::ClaimContext;
use crate::sync::stream::{append_completed_event, stream_tip};

// SERIALIZATION-DOC: camelCase fields preserve the persisted hub identity contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[doc = "Serialized response used to persist one coordination hub identity."]
struct HubConfigResponse {
    hub: String,
    node_id: String,
    node_name: String,
    default_lane: String,
    created_at: String,
}

impl TryFrom<HubConfigResponse> for HubConfig {
    type Error = CoordinationError;

    fn try_from(response: HubConfigResponse) -> Result<Self> {
        Ok(Self {
            hub: response.hub.parse()?,
            node_id: NodeId::parse(response.node_id)?,
            node_name: NodeName::parse(response.node_name)?,
            default_lane: response.default_lane.parse()?,
            created_at: CoordinationTimestamp::parse(&response.created_at)?,
        })
    }
}

impl From<&HubConfig> for HubConfigResponse {
    fn from(config: &HubConfig) -> Self {
        Self {
            hub: config.hub.as_str().to_owned(),
            node_id: config.node_id.as_str().to_owned(),
            node_name: config.node_name.as_str().to_owned(),
            default_lane: config.default_lane.as_str().to_owned(),
            created_at: config.created_at.as_str().to_owned(),
        }
    }
}

pub(super) fn decode_hub_config(raw: &str) -> Result<HubConfig> {
    let response: HubConfigResponse = serde_json::from_str(raw)?;
    response.try_into()
}

pub(super) fn encode_hub_config(config: &HubConfig) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec_pretty(&HubConfigResponse::from(config))?)
}

pub(super) fn normalize_owns_paths(
    repo_root: &CoordinationRepoRoot,
    entries: &[ClaimPath],
) -> Result<Vec<ClaimPath>> {
    let repo_root = repo_root.as_path();
    let mut paths = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in entries {
        let normalized = entry.as_str().trim().replace('\\', "/");
        if normalized.is_empty() {
            continue;
        }
        let is_glob =
            normalized.contains('*') || normalized.contains('?') || normalized.contains('[');
        let looks_like_directory = normalized.ends_with('/');
        if let Some(directory_part) = normalized
            .strip_suffix("/**")
            .filter(|directory| !directory.contains(['*', '?', '[']))
        {
            let directory = repo_root.join(directory_part);
            if directory.is_dir() {
                walk_files(&directory, repo_root, &mut paths, &mut seen)?;
            } else {
                push_normalized(directory_part, &mut paths, &mut seen)?;
            }
        } else if is_glob {
            let pattern = repo_root
                .join(&normalized)
                .to_string_lossy()
                .replace('\\', "/");
            let mut matched = false;
            for entry in glob::glob(&pattern)? {
                let path = entry?;
                if path.is_file() {
                    matched = true;
                    push_relative(repo_root, &path, &mut paths, &mut seen)?;
                }
            }
            if !matched {
                let literal = normalized.trim_end_matches("/**").trim_end_matches('*');
                push_normalized(literal, &mut paths, &mut seen)?;
            }
        } else if looks_like_directory || repo_root.join(&normalized).is_dir() {
            let directory = repo_root.join(normalized.trim_end_matches('/'));
            if directory.is_dir() {
                walk_files(&directory, repo_root, &mut paths, &mut seen)?;
            }
        } else {
            push_normalized(&normalized, &mut paths, &mut seen)?;
        }
    }
    Ok(paths)
}

fn push_normalized(
    value: &str,
    paths: &mut Vec<ClaimPath>,
    seen: &mut BTreeSet<String>,
) -> Result<()> {
    if !value.is_empty() && seen.insert(value.to_owned()) {
        paths.push(ClaimPath::parse(value)?);
    }
    Ok(())
}

fn push_relative(
    root: &Path,
    path: &Path,
    paths: &mut Vec<ClaimPath>,
    seen: &mut BTreeSet<String>,
) -> Result<()> {
    if let Ok(relative) = path.strip_prefix(root) {
        let normalized = relative.to_string_lossy().replace('\\', "/");
        push_normalized(&normalized, paths, seen)?;
    }
    Ok(())
}

fn walk_files(
    directory: &Path,
    root: &Path,
    paths: &mut Vec<ClaimPath>,
    seen: &mut BTreeSet<String>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            if matches!(
                name.to_string_lossy().as_ref(),
                "target" | "node_modules" | ".git"
            ) {
                continue;
            }
            walk_files(&path, root, paths, seen)?;
        } else if path.is_file() {
            push_relative(root, &path, paths, seen)?;
        }
    }
    Ok(())
}

pub(super) struct EventContextRefs<'a> {
    pub claim: &'a ClaimContext,
    pub caller: &'a CallerContext,
}

pub(super) struct AppendEventArgs<'a> {
    pub lane: &'a LaneId,
    pub kind: CoordinationEventKind,
    pub paths: Option<Vec<ClaimPath>>,
    pub reason: Option<ClaimReason>,
    pub context: Option<EventContextRefs<'a>>,
    pub metadata: EventMetadata,
}

#[derive(Default)]
pub(super) struct EventMetadata {
    pub to: Option<LaneId>,
    pub body: Option<CoordinationMessageBody>,
    pub message_id: Option<ClaimEventId>,
}

pub(super) fn append_event(hub: &Hub, args: AppendEventArgs<'_>) -> Result<HubEventResponse> {
    let tip = stream_tip(hub.root.as_path(), &hub.config.node_id, args.lane)?;
    let writer = WriterId::new(&hub.config.node_id, args.lane);
    let seq = tip.as_ref().map_or(1, |event| event.seq + 1);
    let prev_event_id = tip.as_ref().map(|event| event.id.clone());
    let prev_hash = tip.as_ref().map(|event| event.hash.clone());
    let mut event = HubEventResponse {
        id: random_event_id(),
        schema: 1,
        hub: hub.config.hub.as_str().to_owned(),
        node_id: hub.config.node_id.as_str().to_owned(),
        node_name: hub.config.node_name.as_str().to_owned(),
        lane: args.lane.as_str().to_owned(),
        writer: writer.as_str().to_owned(),
        kind: args.kind.as_str().to_owned(),
        ts: now_iso()?.into_string(),
        seq,
        prev_event_id,
        prev_hash,
        hash: String::new(),
        to: args.metadata.to.map(|lane| lane.as_str().to_owned()),
        body: args.metadata.body.map(CoordinationMessageBody::into_string),
        message_id: args.metadata.message_id.map(ClaimEventId::into_string),
        paths: args
            .paths
            .map(|paths| paths.into_iter().map(ClaimPath::into_string).collect()),
        reason: args.reason.map(ClaimReason::into_string),
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
        context: args
            .context
            .map(|context| claim_context_to_json(context.claim, context.caller)),
    };
    event.hash = crate::events::hash_for_event(&event)?;
    append_completed_event(hub.root.as_path(), &hub.config.node_id, args.lane, &event)?;
    Ok(event)
}

pub(super) fn now_iso() -> Result<CoordinationTimestamp> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = now.as_secs();
    let millis = now.subsec_millis();
    let days = seconds / 86_400;
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(i64::MAX));
    let remainder = seconds % 86_400;
    let (hour, minute, second) = (remainder / 3_600, (remainder % 3_600) / 60, remainder % 60);
    Ok(CoordinationTimestamp::try_from(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
    ))?)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = u64::try_from(z - era * 146_097).unwrap_or_default();
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = i64::try_from(year_of_era).unwrap_or_default() + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = u32::try_from(day_of_year - (153 * month_position + 2) / 5 + 1).unwrap_or_default();
    let month = if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    };
    let month = u32::try_from(month).unwrap_or_default();
    (if month <= 2 { year + 1 } else { year }, month, day)
}

fn claim_context_to_json(context: &ClaimContext, caller: &CallerContext) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    macro_rules! set {
        ($key:literal, $field:expr) => {
            if let Some(value) = &$field {
                map.insert(
                    $key.to_owned(),
                    serde_json::Value::String(value.to_string()),
                );
            }
        };
    }
    set!("projectId", context.project_id);
    set!("gitRemote", context.git_remote);
    set!("repoRoot", context.repo_root);
    set!("worktreeRoot", context.worktree_root);
    set!("branch", context.branch);
    set!("codexThreadId", context.codex_thread_id);
    set!("codexSessionId", context.codex_session_id);
    set!("claimGroup", context.claim_group);
    if let Some(lock_kind) = context.lock_kind {
        map.insert(
            "lockKind".to_owned(),
            serde_json::Value::String(lock_kind.as_str().to_owned()),
        );
    }
    if let Some(operation) = context.operation {
        map.insert(
            "operation".to_owned(),
            serde_json::Value::String(operation.as_str().to_owned()),
        );
    }
    if let Some(commit) = &caller.commit {
        map.insert(
            "commit".to_owned(),
            serde_json::Value::String(commit.as_str().to_owned()),
        );
    }
    serde_json::Value::Object(map)
}

fn random_event_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|before_epoch| before_epoch.duration());
    format!(
        "evt_{:032x}",
        now.as_nanos() ^ (u128::from(std::process::id()) << 32)
    )
}

#[cfg(test)]
mod tests {
    use super::decode_hub_config;

    #[test]
    fn decode_hub_config_rejects_bad_hub_identity() {
        let bad = r#"{"hub":"UPPERCASE","nodeId":"node_a","nodeName":"node-a","defaultLane":"arc-16","createdAt":"2026-07-19T00:00:00.000Z"}"#;
        let outcome = decode_hub_config(bad);
        assert!(
            outcome.is_err(),
            "HubConfig with invalid hub should be rejected"
        );
        if let Err(error) = outcome {
            let message = error.to_string();
            assert!(message.contains("coordination decode error"));
        }
    }

    #[test]
    fn decode_hub_config_rejects_bad_node_id() {
        let bad = r#"{"hub":"valid-hub","nodeId":"invalid node","nodeName":"node-a","defaultLane":"arc-16","createdAt":"2026-07-19T00:00:00.000Z"}"#;
        let outcome = decode_hub_config(bad);
        assert!(
            outcome.is_err(),
            "HubConfig with invalid nodeId should be rejected"
        );
        if let Err(error) = outcome {
            let message = error.to_string();
            assert!(message.contains("coordination decode error"));
        }
    }

    #[test]
    fn decode_hub_config_rejects_bad_default_lane() {
        let bad = r#"{"hub":"valid-hub","nodeId":"node_a","nodeName":"node-a","defaultLane":"UPPER","createdAt":"2026-07-19T00:00:00.000Z"}"#;
        let outcome = decode_hub_config(bad);
        assert!(
            outcome.is_err(),
            "HubConfig with invalid lane should be rejected"
        );
        if let Err(error) = outcome {
            let message = error.to_string();
            assert!(message.contains("coordination decode error"));
        }
    }

    #[test]
    fn decode_hub_config_rejects_blank_created_at() {
        let bad = r#"{"hub":"valid-hub","nodeId":"node_a","nodeName":"node-a","defaultLane":"arc-16","createdAt":""}"#;
        let outcome = decode_hub_config(bad);
        assert!(
            outcome.is_err(),
            "HubConfig with blank timestamp should be rejected"
        );
        if let Err(error) = outcome {
            let message = error.to_string();
            assert!(message.contains("coordination decode error"));
        }
    }
}
