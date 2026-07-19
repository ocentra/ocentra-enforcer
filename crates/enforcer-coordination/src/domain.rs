//! Coordination hub identity.
//!
//! Filesystem and environment translation lives in [`boundary`]; this module
//! contains only the validated identity held by command and stream logic.

use enforcer_domain::coordination_types::{CoordinationTimestamp, NodeId, NodeName};
use enforcer_domain::ids::{HubName, LaneId};

pub mod boundary;

/// The persisted, validated identity of one coordination hub node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubConfig {
    pub hub: HubName,
    pub node_id: NodeId,
    pub node_name: NodeName,
    pub default_lane: LaneId,
    pub created_at: CoordinationTimestamp,
}
