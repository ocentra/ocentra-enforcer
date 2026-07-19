//! Environment and filesystem boundary for coordination ledger paths.
//!
//! BOUNDARY-INVARIANT: raw environment strings and filesystem paths are
//! resolved here before coordination domain logic consumes them.
//! BOUNDARY-TEST: blank path overrides and invalid hub identities are rejected
//! by the colocated negative tests.
//! boundaryOwnerNote: enforcer-coordination owns ledger path resolution.

use std::env;
use std::path::{Path, PathBuf};

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::coordination_types::{CoordinationLedgerRoot, NodeId};
use enforcer_domain::ids::{HubName, LaneId};

use crate::error::Result;

/// Resolve raw environment or caller overrides into a validated absolute ledger root.
pub fn resolve_ledger_root(
    hub_override: Option<&str>,
    root_override: Option<&str>,
) -> Result<CoordinationLedgerRoot> {
    let optional_env = |name| env::var(name).ok();
    if let Some(explicit) = root_override
        .map(str::to_owned)
        .or_else(|| optional_env("LEDGER_ROOT"))
        .or_else(|| optional_env("OCENTRA_COORDINATION_ROOT"))
    {
        return Ok(CoordinationLedgerRoot::parse(&absolute(&explicit)?)?);
    }
    let raw_hub = hub_override
        .map(str::to_owned)
        .or_else(|| optional_env("OCENTRA_COORDINATION_HUB"))
        .or_else(|| optional_env("OCENTRA_ENFORCER_HUB"))
        .unwrap_or_else(|| "ocentra-parent".to_owned());
    let hub: HubName = raw_hub.parse()?;
    Ok(CoordinationLedgerRoot::parse(
        &ledger_home()?.join(hub.as_str()),
    )?)
}

fn ledger_home() -> Result<PathBuf> {
    let raw = env::var("OCENTRA_LEDGER_HOME")
        .or_else(|_| env::var("OCENTRA_COORDINATION_HOME"))
        .or_else(|_| env::var("LEDGER_HOME"))
        .unwrap_or_else(|_| ".ledger".to_owned());
    absolute(&raw)
}

fn absolute(raw: &str) -> Result<PathBuf> {
    if raw.trim().is_empty() {
        return Err(DecodeError::new(
            "coordinationLedgerRoot",
            "expected a non-blank path override",
        )
        .into());
    }
    let path = Path::new(raw);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

/// Return the directory containing the persisted node identity.
pub(crate) fn identity_dir(root: &Path) -> PathBuf {
    root.join("identity")
}

/// Return the persisted node identity file.
pub(crate) fn identity_path(root: &Path) -> PathBuf {
    identity_dir(root).join("node.json")
}

/// Return the live event-stream directory.
pub(crate) fn streams_dir(root: &Path) -> PathBuf {
    root.join("streams")
}

/// Return the archived event-stream directory.
pub(crate) fn archive_streams_dir(root: &Path) -> PathBuf {
    root.join("archive").join("streams")
}

/// Return the archive directory for one logical stream.
pub(crate) fn archived_stream_dir(root: &Path, stream_name: &str) -> PathBuf {
    archive_streams_dir(root).join(stream_name)
}

/// Return the live stream file for one node and lane.
pub(crate) fn stream_path(root: &Path, node_id: &NodeId, lane: &LaneId) -> PathBuf {
    streams_dir(root).join(format!("{node_id}.{}.ndjson", lane.as_str()))
}

/// Return the exclusive append lock for one node and lane stream.
pub(crate) fn lock_path(root: &Path, node_id: &NodeId, lane: &LaneId) -> PathBuf {
    streams_dir(root).join(format!("{node_id}.{}.lock", lane.as_str()))
}

#[cfg(test)]
mod tests {
    use super::resolve_ledger_root;

    #[test]
    fn blank_explicit_root_is_rejected_at_the_boundary(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let error = match resolve_ledger_root(None, Some("")) {
            Err(error) => error,
            Ok(root) => return Err(format!("blank root unexpectedly resolved to {root}").into()),
        };
        assert_eq!(
            error.to_string(),
            "coordination decode error: decode/validation failed at `coordinationLedgerRoot`: expected a non-blank path override"
        );
        Ok(())
    }

    #[test]
    fn invalid_hub_override_is_rejected_at_the_boundary(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let error = match resolve_ledger_root(Some("../other-hub"), None) {
            Err(error) => error,
            Ok(root) => {
                return Err(format!("path-shaped hub unexpectedly resolved to {root}").into())
            }
        };
        assert_eq!(
            error.to_string(),
            "coordination decode error: decode/validation failed at `hubName`: expected lowercase kebab-case (e.g. `enforcer-rust-build`)"
        );
        Ok(())
    }
}
