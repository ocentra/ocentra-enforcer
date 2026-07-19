//! Crate-local error type for `enforcer-plan`.
//!
//! Owned by the SKELETON (arc-20), not by the feature packs (b01-b06,
//! x05) that mount their modules under this crate. Feature modules add
//! their own error variants here rather than inventing a second error
//! enum, so callers of this crate see exactly one failure type.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::ids::LaneId;
use enforcer_domain::plan_types::{PlanArtifactPath, PlanDiagnosticDetail, PlanImportCount};

/// Failures the plan scaffolder / PLAN-* structure validators can raise.
///
/// Deliberately starts minimal (I/O only). Feature packs extend this enum
/// with their own variants as they land (e.g. a `TemplateDrift` variant
/// for b03, a `ScaffoldRefused` variant for b01's `--force` contract) —
/// this is the one place those variants belong, not a per-module error
/// type.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    /// A typed lesson seed field failed validation during boundary conversion.
    #[error("invalid lesson seed record: {0}")]
    SeedDecode(#[from] DecodeError),

    /// A plan-doc or template path could not be read from disk.
    #[error("failed to read `{path}`: {reason}")]
    Io {
        // BRAND-INVARIANT: raw display-only string, verbatim from the
        // triggering `std::path::Path::display()` call; never parsed back
        // into a filesystem path or re-opened, so it is not branded as
        // `enforcer_domain::paths::RelPath` (that type is for paths a
        // caller will act on, not for error-message text).
        /// Path that failed to read, rendered for the error message only.
        path: PlanArtifactPath,
        // BRAND-INVARIANT: raw display-only string, verbatim from the
        // underlying `std::io::Error`'s `Display` output; free-form
        // diagnostic text, not a structured/matched-on failure reason.
        /// Underlying I/O failure description.
        reason: PlanDiagnosticDetail,
    },

    /// (b01) A caller passed a plan name that fails the `PlanName` brand
    /// (lowercase kebab-case) before any filesystem I/O ran.
    #[error("invalid plan name `{raw}`: expected lowercase kebab-case")]
    InvalidPlanName {
        // BRAND-INVARIANT: raw display-only string, the rejected input
        // verbatim, so the error message can show what was rejected; never
        // reused as a constructed `PlanName`.
        /// The rejected raw plan-name input.
        raw: String,
    },

    /// (b01) `scaffold_plan` refuses to overwrite an existing plan
    /// directory unless the caller passes `force`.
    #[error("plan directory already exists at `{path}` (pass `force` to overwrite)")]
    PlanAlreadyExists {
        // BRAND-INVARIANT: raw display-only string, verbatim from
        // `std::path::Path::display()`; error-message text only.
        /// Plan-directory path that already exists.
        path: PlanArtifactPath,
    },

    /// (b04) A workpack's `deps:` field references a workpack id that is
    /// absent from the frontier input, or the dep graph contains a cycle —
    /// either way the frontier computation cannot proceed deterministically.
    #[error("plan graph error: {reason}")]
    GraphInvalid {
        /// Human-readable description (unknown dep id, or a cycle
        /// participant list); free-form diagnostic text, not matched on.
        reason: PlanDiagnosticDetail,
    },

    /// (b04) The underlying `enforcer-coordination` claim/release/closeout
    /// call failed. Wrapped (not silently swallowed) so orchestrator
    /// callers see the coordination crate's own `Display` text.
    #[error("coordination error: {0}")]
    Coordination(#[from] enforcer_coordination::error::CoordinationError),

    /// (b04, L14/L16) A `tick()` call would end with lanes still in flight
    /// and no next-wake scheduled. Ending a turn in this fragile state is a
    /// FAILURE MODE, not a rest state — this variant makes that failure
    /// mechanically unrepresentable as a silent `Ok(())`.
    #[error(
        "idle-without-watchdog: {in_flight_lanes} lane(s) in flight but no next tick was armed"
    )]
    IdleWithoutWatchdog {
        /// Count of lanes still in flight when the tick ended.
        in_flight_lanes: PlanImportCount,
    },

    /// (b04, zero-trust integration) A lane's `done` mail was rejected
    /// because independent re-verification (scope diff against `owns:`,
    /// re-run proof) did not corroborate the claim — the done-claim is
    /// tampered or premature and must never be trusted on faith.
    #[error("done-claim rejected for lane `{lane}`: {reason}")]
    DoneClaimRejected {
        /// The lane whose done-claim failed independent verification.
        lane: LaneId,
        /// Why verification failed (free-form diagnostic text).
        reason: PlanDiagnosticDetail,
    },
}

/// Result alias for `enforcer-plan` fallible operations.
pub type PlanResult<T> = std::result::Result<T, PlanError>;
