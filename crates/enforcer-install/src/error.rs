//! Typed error surface for `enforcer-install`. Every fallible operation in
//! this crate (plan/apply/verify, managed-block edits, backups, binary
//! resolution) returns [`InstallError`] — never a bare `String`/`anyhow`
//! leak across the crate boundary, so `enforcer-cli` can pattern-match a
//! stable set of variants into the exit-code taxonomy
//! (`enforcer_core::exit_codes::ExitCode`) instead of guessing from text.

/// Failure modes an adapter's `plan`/`apply`/`verify` cycle, or a shared
/// helper (managed-block, backup, distribution), can raise.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum InstallError {
    /// The target harness's config file exists but is not valid for the
    /// format the adapter expects (malformed JSON/TOML, or missing the
    /// structure the adapter needs to locate its managed section).
    #[error("malformed harness config at `{path}`: {reason}")]
    MalformedConfig {
        /// Path to the offending config file.
        path: String,
        /// Human-readable reason the config could not be parsed/located.
        reason: String,
    },

    /// A read or write against the filesystem failed outside the
    /// dry-run path (dry-run never touches disk; see [`crate::cli_contract`]).
    #[error("filesystem operation failed at `{path}`: {reason}")]
    Io {
        /// Path the failing operation targeted.
        path: String,
        /// Underlying I/O failure description.
        reason: String,
    },

    /// A managed block marker pair (`begin`/`end`) was not found, or was
    /// found more than once, in a text config an adapter edits in place.
    #[error("managed block `{marker}` invalid in `{path}`: {reason}")]
    ManagedBlockInvalid {
        /// Path to the file containing (or missing) the managed block.
        path: String,
        /// The marker name the adapter searched for.
        marker: String,
        /// Why the block is invalid (missing / duplicated / unterminated).
        reason: String,
    },

    /// No backup exists to restore from, or writing a backup failed.
    #[error("backup operation failed for `{path}`: {reason}")]
    BackupFailed {
        /// Path the backup was for.
        path: String,
        /// Underlying failure description.
        reason: String,
    },

    /// The current OS/arch has no published binary in the release matrix
    /// (win/mac/linux incl. musl + apple-silicon).
    #[error("no released binary for target `{target}`")]
    UnsupportedTarget {
        /// The unresolved target triple/label.
        target: String,
    },

    /// The binary-distribution download or checksum-verification step
    /// failed.
    #[error("binary distribution failed for `{target}`: {reason}")]
    DistributionFailed {
        /// Target the download was for.
        target: String,
        /// Underlying failure description.
        reason: String,
    },

    /// A `verify(ctx)` check found the installed state does not match what
    /// `apply(report)` was supposed to have produced (post-install drift or
    /// a partially-applied change).
    #[error("post-install verification failed: {0}")]
    VerificationFailed(String),

    /// A caller-supplied `--only <harness>` (or manifest) adapter key does
    /// not match any registered [`crate::core::HarnessAdapter`]. Fail-closed:
    /// an unrecognized adapter id is a typed error, never a silent skip
    /// (workpack c01 acceptance row).
    #[error("unknown harness adapter id `{id}`; known adapters: {known}")]
    UnknownAdapter {
        /// The unrecognized adapter key the caller supplied.
        id: String,
        /// Comma-joined list of the adapter keys that ARE registered, for
        /// the terse `Fix:`-style hint surfaced by `enforcer-cli`.
        known: String,
    },

    /// A skill-asset doctor/verify check (RUST_ARCHITECTURE "skill-asset
    /// VALIDATOR fold-in") found a declared asset missing on disk, or the
    /// `.codex-plugin/plugin.json` publish contract (`plugin.skills ==
    /// "./skills/"`) broken. Distinct from [`InstallError::VerificationFailed`]
    /// so callers can pattern-match the skill-asset family specifically.
    #[error("skill-asset check `{check}` failed for `{path}`: {reason}")]
    SkillAssetInvalid {
        /// Which skill-asset check failed (e.g. "skill-md-exists",
        /// "plugin-skills-path").
        check: String,
        /// Path the check was evaluating.
        path: String,
        /// Why the check failed.
        reason: String,
    },
}

/// Result alias for `enforcer-install` operations.
pub type InstallResult<T> = std::result::Result<T, InstallError>;
