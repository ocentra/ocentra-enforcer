//! Typed error surface for `enforcer-install`. Every fallible operation in
//! this crate (plan/apply/verify, managed-block edits, backups, binary
//! resolution) returns [`InstallError`] — never a bare `String`/`anyhow`
//! leak across the crate boundary, so `enforcer-cli` can pattern-match a
//! stable set of variants into the exit-code taxonomy
//! (`enforcer_domain::core_types::ExitCode`) instead of guessing from text.

//! BOUNDARY-INVARIANT: external error text is represented as typed install errors.
//!
/// Failure modes an adapter's `plan`/`apply`/`verify` cycle, or a shared
/// helper (managed-block, backup, distribution), can raise.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum InstallError {
    /// A process, file, or transport boundary supplied a value that failed
    /// canonical installation-domain validation.
    #[error("invalid installation boundary value: {0}")]
    InvalidDomain(#[from] enforcer_domain::boundary::decode_error::DecodeError),

    /// The target harness's config file exists but is not valid for the
    /// format the adapter expects (malformed JSON/TOML, or missing the
    /// structure the adapter needs to locate its managed section).
    #[error("malformed harness config at `{path}`: {reason}")]
    MalformedConfig {
        /// Path to the offending config file.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        path: String,
        /// Human-readable reason the config could not be parsed/located.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        reason: String,
    },

    /// A read or write against the filesystem failed outside the
    /// dry-run path (dry-run never touches disk; see [`crate::request_context`]).
    #[error("filesystem operation failed at `{path}`: {reason}")]
    Io {
        /// Path the failing operation targeted.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        path: String,
        /// Underlying I/O failure description.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        reason: String,
    },

    /// A managed block marker pair (`begin`/`end`) was not found, or was
    /// found more than once, in a text config an adapter edits in place.
    #[error("managed block `{marker}` invalid in `{path}`: {reason}")]
    ManagedBlockInvalid {
        /// Path to the file containing (or missing) the managed block.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        path: String,
        /// The marker name the adapter searched for.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        marker: String,
        /// Why the block is invalid (missing / duplicated / unterminated).
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        reason: String,
    },

    /// No backup exists to restore from, or writing a backup failed.
    #[error("backup operation failed for `{path}`: {reason}")]
    BackupFailed {
        /// Path the backup was for.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        path: String,
        /// Underlying failure description.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        reason: String,
    },

    /// The current OS/arch has no published binary in the release matrix
    /// (win/mac/linux incl. musl + apple-silicon).
    #[error("no released binary for target `{target}`")]
    UnsupportedTarget {
        /// The unresolved target triple/label.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        target: String,
    },

    /// The binary-distribution download or checksum-verification step
    /// failed.
    #[error("binary distribution failed for `{target}`: {reason}")]
    DistributionFailed {
        /// Target the download was for.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        target: String,
        /// Underlying failure description.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
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
        id: enforcer_domain::ids::HarnessId,
        /// Comma-joined list of the adapter keys that ARE registered, for
        /// the terse `Fix:`-style hint surfaced by `enforcer-cli`.
        known: enforcer_domain::install_types::KnownHarnesses,
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
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        check: String,
        /// Path the check was evaluating.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        path: String,
        /// Why the check failed.
        // BRAND-INVARIANT: this private raw field is module-controlled fixture or diagnostic payload and is never accepted as a domain identity.
        reason: String,
    },
}

/// Result alias for `enforcer-install` operations.
pub type InstallResult<T> = std::result::Result<T, InstallError>;
