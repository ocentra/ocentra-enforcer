//! Crate-local error type for `enforcer-plan`.
//!
//! Owned by the SKELETON (arc-20), not by the feature packs (b01-b06,
//! x05) that mount their modules under this crate. Feature modules add
//! their own error variants here rather than inventing a second error
//! enum, so callers of this crate see exactly one failure type.

/// Failures the plan scaffolder / PLAN-* structure validators can raise.
///
/// Deliberately starts minimal (I/O only). Feature packs extend this enum
/// with their own variants as they land (e.g. a `TemplateDrift` variant
/// for b03, a `ScaffoldRefused` variant for b01's `--force` contract) —
/// this is the one place those variants belong, not a per-module error
/// type.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    /// A plan-doc or template path could not be read from disk.
    #[error("failed to read `{path}`: {reason}")]
    Io {
        // BRAND-INVARIANT: raw display-only string, verbatim from the
        // triggering `std::path::Path::display()` call; never parsed back
        // into a filesystem path or re-opened, so it is not branded as
        // `enforcer_domain::paths::RelPath` (that type is for paths a
        // caller will act on, not for error-message text).
        /// Path that failed to read, rendered for the error message only.
        path: String,
        // BRAND-INVARIANT: raw display-only string, verbatim from the
        // underlying `std::io::Error`'s `Display` output; free-form
        // diagnostic text, not a structured/matched-on failure reason.
        /// Underlying I/O failure description.
        reason: String,
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
        path: String,
    },
}

/// Result alias for `enforcer-plan` fallible operations.
pub type PlanResult<T> = std::result::Result<T, PlanError>;
