//! Serialized implementation and proof DTOs.
//!
//! BOUNDARY-INVARIANT: implementation and executable-proof coverage are
//! independent truth dimensions and never inferred from CP08 decomposition.
//! NEGATIVE-TEST: crates/enforcer-rules/tests/cyberskills_disposition/negative.rs
//! rejects contradictory implementation and proof states.
//! ROUNDTRIP-TEST: crates/enforcer-rules/tests/cyberskills_disposition/manifest.rs
//! contains implementation, native, and executable-proof codec cycles.

use serde::{Deserialize, Serialize};

use super::super::types::{ComponentIdListEnvelope, CoverageLevel};

/// Independent implementation and executable-proof truth.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImplementationTruthDto {
    pub native: NativeImplementationDto,
    pub executable_proof: ExecutableProofDto,
}

/// Native implementation coverage and implemented component IDs.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeImplementationDto {
    pub coverage: CoverageLevel,
    pub component_ids: ComponentIdListEnvelope,
}

/// Executable proof coverage independent of decomposition evidence.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutableProofDto {
    pub coverage: CoverageLevel,
}
