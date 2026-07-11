//! Coordination hub identity, root resolution, and stream paths.
//!
//! Ported from `src/coordination/vendor/{domain,identity,paths,root}.js`.
//! `HubName`/`LaneId` are re-used from `enforcer-domain` (already branded
//! there) rather than re-declared; the additional identifiers here
//! (`NodeId`, `NodeName`, `WriterId`) are coordination-local because they are
//! not needed outside this crate.

use std::env;
use std::path::{Path, PathBuf};

use enforcer_domain::ids::{HubName, LaneId};
use serde::{Deserialize, Serialize};

use crate::error::CoordinationError;

/// A coordination node identifier, e.g. `node_<uuid-no-dashes>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(raw: impl Into<String>) -> Result<Self, CoordinationError> {
        let raw = raw.into();
        if is_identity_like(&raw, 1, 96) {
            Ok(Self(raw))
        } else {
            Err(CoordinationError::invalid("nodeId", raw))
        }
    }

    /// Generate a fresh random node id (`node_<32 hex chars>`), analogous to
    /// `identity.js#randomNodeId` (which used `randomUUID`).
    pub fn random() -> Self {
        Self(format!("node_{}", random_hex(32)))
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A coordination node's display name (defaults to a sanitized hostname).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeName(String);

impl NodeName {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(raw: impl Into<String>) -> Result<Self, CoordinationError> {
        let raw = raw.into();
        if is_identity_like(&raw, 1, 96) {
            Ok(Self(raw))
        } else {
            Err(CoordinationError::invalid("nodeName", raw))
        }
    }

    /// Sanitize an arbitrary hostname string into a valid `NodeName`, per
    /// `identity.js#displayHostname` (`replaceAll(/[^A-Za-z0-9._-]/g, "_")`).
    pub fn sanitize_hostname(raw: &str) -> Self {
        let sanitized: String = raw
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let sanitized = if sanitized.is_empty() {
            "unknown-host".to_owned()
        } else {
            sanitized.chars().take(96).collect()
        };
        Self(sanitized)
    }
}

impl std::fmt::Display for NodeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// `<nodeId>.<lane>` — the writer identity attached to every appended event.
/// Ported from `domain.js#writerId`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WriterId(String);

impl WriterId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn new(node_id: &NodeId, lane: &LaneId) -> Self {
        Self(format!("{node_id}.{}", lane.as_str()))
    }

    /// The `nodeId` prefix of this writer id, used by closeout's `nodeId`
    /// scope filter (`writer.startsWith(`${nodeId}.`)`).
    pub fn node_id_prefix(&self) -> &str {
        self.0.split('.').next().unwrap_or(&self.0)
    }
}

impl std::fmt::Display for WriterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_identity_like(raw: &str, min: usize, max: usize) -> bool {
    let len = raw.chars().count();
    len >= min
        && len <= max
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// A tiny dependency-free hex-string generator so this crate does not need a
/// `uuid`/`rand` dependency solely for node-id generation. Not cryptographic;
/// node ids are identity labels, not secrets.
fn random_hex(len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
        ^ (std::process::id() as u128) << 64;
    let mut state = seed | 1;
    let mut out = String::with_capacity(len);
    while out.len() < len {
        // xorshift64-ish mix, good enough for a non-cryptographic label.
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        out.push_str(&format!("{:016x}", state as u64));
    }
    out.truncate(len);
    out
}

/// The persisted hub identity record (`identity/node.json`).
/// Ported from `domain.js#HubConfigSchema`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubConfig {
    pub hub: HubName,
    pub node_id: NodeId,
    pub node_name: NodeName,
    pub default_lane: LaneId,
    pub created_at: String,
}

/// Resolve the coordination ledger root directory.
/// Ported from `root.js#resolveLedgerRoot`/`resolveLedgerHome`.
///
/// Env precedence (matches the vendored JS exactly):
/// - `LEDGER_ROOT` / `OCENTRA_COORDINATION_ROOT` → used verbatim as the root.
/// - otherwise `<ledger-home>/<hub>` where `<hub>` comes from
///   `OCENTRA_COORDINATION_HUB` / `OCENTRA_ENFORCER_HUB` (default
///   `ocentra-parent`) and `<ledger-home>` comes from
///   `OCENTRA_LEDGER_HOME` / `OCENTRA_COORDINATION_HOME` / `LEDGER_HOME`
///   (default: `<pack-root>/.ledger`, here approximated as `./.ledger`
///   relative to the current working directory since this crate has no
///   notion of "pack root").
pub fn resolve_ledger_root(hub_override: Option<&str>, root_override: Option<&str>) -> PathBuf {
    if let Some(explicit) = root_override
        .map(str::to_owned)
        .or_else(|| env::var("LEDGER_ROOT").ok())
        .or_else(|| env::var("OCENTRA_COORDINATION_ROOT").ok())
    {
        return absolute(&explicit);
    }
    let hub = hub_override
        .map(str::to_owned)
        .or_else(|| env::var("OCENTRA_COORDINATION_HUB").ok())
        .or_else(|| env::var("OCENTRA_ENFORCER_HUB").ok())
        .unwrap_or_else(|| "ocentra-parent".to_owned());
    ledger_home().join(hub)
}

fn ledger_home() -> PathBuf {
    let raw = env::var("OCENTRA_LEDGER_HOME")
        .or_else(|_| env::var("OCENTRA_COORDINATION_HOME"))
        .or_else(|_| env::var("LEDGER_HOME"))
        .unwrap_or_else(|_| ".ledger".to_owned());
    absolute(&raw)
}

fn absolute(raw: &str) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

/// `<root>/identity`
pub fn identity_dir(root: &Path) -> PathBuf {
    root.join("identity")
}

/// `<root>/identity/node.json`
pub fn identity_path(root: &Path) -> PathBuf {
    identity_dir(root).join("node.json")
}

/// `<root>/streams`
pub fn streams_dir(root: &Path) -> PathBuf {
    root.join("streams")
}

/// `<root>/archive/streams`
pub fn archive_streams_dir(root: &Path) -> PathBuf {
    root.join("archive").join("streams")
}

/// `<root>/archive/streams/<stream_name>`
pub fn archived_stream_dir(root: &Path, stream_name: &str) -> PathBuf {
    archive_streams_dir(root).join(stream_name)
}

/// `<root>/streams/<writer>.ndjson`
pub fn stream_path(root: &Path, node_id: &NodeId, lane: &LaneId) -> PathBuf {
    streams_dir(root).join(format!("{node_id}.{}.ndjson", lane.as_str()))
}

/// `<root>/streams/<writer>.lock`
pub fn lock_path(root: &Path, node_id: &NodeId, lane: &LaneId) -> PathBuf {
    streams_dir(root).join(format!("{node_id}.{}.lock", lane.as_str()))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn writer_id_formats_and_extracts_node_prefix() {
        let node = NodeId::parse("node_abc123").expect("valid node id");
        let lane: LaneId = "arc-16".parse().expect("valid lane id");
        let writer = WriterId::new(&node, &lane);
        assert_eq!(writer.as_str(), "node_abc123.arc-16");
        assert_eq!(writer.node_id_prefix(), "node_abc123");
    }

    #[test]
    fn sanitize_hostname_replaces_invalid_characters() {
        let name = NodeName::sanitize_hostname("My PC (office)!");
        assert_eq!(name.as_str(), "My_PC__office__");
    }

    #[test]
    fn resolve_ledger_root_prefers_explicit_root_override() {
        let root = resolve_ledger_root(Some("some-hub"), Some("C:/tmp/explicit-root"));
        assert!(
            root.ends_with("explicit-root") || root.to_string_lossy().contains("explicit-root")
        );
    }

    #[test]
    fn resolve_ledger_root_joins_home_and_hub_when_no_explicit_root() {
        let root = resolve_ledger_root(Some("enforcer-rust-build"), None);
        assert!(root.ends_with("enforcer-rust-build"));
    }

    #[test]
    fn persisted_hub_identity_uses_camel_case_wire_fields(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let raw = serde_json::json!({
            "hub": "ocentra-enforcer",
            "nodeId": "node_7450523d7490414f86992de67525c1c2",
            "nodeName": "GameDev",
            "defaultLane": "codex-proof-migration",
            "createdAt": "2026-06-30T23:19:16.344Z"
        });

        let config: HubConfig = serde_json::from_value(raw)?;
        let rendered = serde_json::to_value(&config)?;

        assert_eq!(config.default_lane.as_str(), "codex-proof-migration");
        assert_eq!(
            rendered.get("nodeId").and_then(serde_json::Value::as_str),
            Some("node_7450523d7490414f86992de67525c1c2")
        );
        assert_eq!(rendered.get("node_id"), None);
        Ok(())
    }
}
