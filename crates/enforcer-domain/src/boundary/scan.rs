//! Transport DTOs for scan-domain values.

// BOUNDARY-INVARIANT: tagged serde wire values are converted immediately into
// closed scan-domain enums before leaving this module.
// boundaryOwnerNote: enforcer-domain owns the shared scan transport decoder.
// Negative malformed and unknown-tag inputs are rejected by serde and covered
// by scan boundary tests.

use crate::scan_types::{Outcome, RouteScope, ScanMode, ScanValidatorCount, SkipReason};
use serde::Deserialize;

#[derive(serde::Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum OutcomeWire {
    Ran { validator_count: ScanValidatorCount },
    Skipped { reason: SkipReason },
}

/// Decode the tagged outcome wire representation.
pub(crate) fn deserialize_outcome<'de, D>(deserializer: D) -> Result<Outcome, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match OutcomeWire::deserialize(deserializer)? {
        OutcomeWire::Ran { validator_count } => Outcome::Ran { validator_count },
        OutcomeWire::Skipped { reason } => Outcome::Skipped { reason },
    })
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ScanModeWire {
    Quick,
    Full,
    Repo,
    Workspace,
    Scoped,
    Diff,
    PlanScan,
}

/// Decode the tagged scan-mode wire representation.
pub(crate) fn deserialize_scan_mode<'de, D>(deserializer: D) -> Result<ScanMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match ScanModeWire::deserialize(deserializer)? {
        ScanModeWire::Quick => ScanMode::Quick,
        ScanModeWire::Full => ScanMode::Full,
        ScanModeWire::Repo => ScanMode::Repo,
        ScanModeWire::Workspace => ScanMode::Workspace,
        ScanModeWire::Scoped => ScanMode::Scoped,
        ScanModeWire::Diff => ScanMode::Diff,
        ScanModeWire::PlanScan => ScanMode::PlanScan,
    })
}

/// Wire projection for the canonical [`RouteScope`] domain enum.
///
/// The projection stays boundary-owned so the domain value does not carry
/// transport tags in its internal representation.  Consumers deserialize the
/// wire shape directly into `RouteScope`; there is no crate-local duplicate
/// route-scope enum.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "root", rename_all = "camelCase")]
enum RouteScopeWire {
    Repo,
    Workspace,
    Crate(crate::paths::RelPath),
    Package(crate::paths::RelPath),
    Folder(crate::paths::RelPath),
    Domain(crate::paths::RelPath),
    Diff,
}

impl serde::Serialize for RouteScope {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wire = match self {
            Self::Repo => RouteScopeWire::Repo,
            Self::Workspace => RouteScopeWire::Workspace,
            Self::Crate(root) => RouteScopeWire::Crate(root.clone()),
            Self::Package(root) => RouteScopeWire::Package(root.clone()),
            Self::Folder(root) => RouteScopeWire::Folder(root.clone()),
            Self::Domain(root) => RouteScopeWire::Domain(root.clone()),
            Self::Diff => RouteScopeWire::Diff,
        };
        wire.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for RouteScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match RouteScopeWire::deserialize(deserializer)? {
            RouteScopeWire::Repo => Self::Repo,
            RouteScopeWire::Workspace => Self::Workspace,
            RouteScopeWire::Crate(root) => Self::Crate(root),
            RouteScopeWire::Package(root) => Self::Package(root),
            RouteScopeWire::Folder(root) => Self::Folder(root),
            RouteScopeWire::Domain(root) => Self::Domain(root),
            RouteScopeWire::Diff => Self::Diff,
        })
    }
}
