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
}

/// Result alias for `enforcer-install` operations.
pub type InstallResult<T> = std::result::Result<T, InstallError>;
