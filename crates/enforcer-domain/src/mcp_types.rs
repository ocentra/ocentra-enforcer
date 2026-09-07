//! Canonical value types shared by MCP transport, routing, and artifact
//! fingerprint consumers.

use crate::{boundary::decode_error::DecodeError, hashes::Sha256};
use std::path::Path;

/// The canonical MCP server identity used by installers and transport.
///
/// This product value belongs to the dependency-light domain crate so that
/// installation does not depend on the MCP transport merely to register it.
pub const SERVER_NAME: &str = "enforcer";

/// JSON-RPC server error codes supported by the MCP transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for RpcErrorCode."]
pub enum RpcErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
}

impl From<RpcErrorCode> for i64 {
    fn from(value: RpcErrorCode) -> Self {
        match value {
            RpcErrorCode::ParseError => -32700,
            RpcErrorCode::InvalidRequest => -32600,
            RpcErrorCode::MethodNotFound => -32601,
            RpcErrorCode::InvalidParams => -32602,
            RpcErrorCode::InternalError => -32603,
        }
    }
}

/// A validated non-empty JSON-RPC error message.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for RpcErrorMessage."]
pub struct RpcErrorMessage {
    #[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
    value: String,
}

impl RpcErrorMessage {
    /// Validate an error message before it enters MCP routing outcomes.
    pub fn try_new(value: &str) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            return Err(DecodeError::new("rpcErrorMessage", "must not be empty"));
        }
        // ALLOC-JUSTIFICATION: the validated error detail outlives the transport buffer.
        Ok(Self {
            value: value.to_owned(),
        })
    }

    /// Stable non-empty fallback for an unexpected boundary formatting failure.
    pub fn fallback() -> Self {
        // ALLOC-JUSTIFICATION: the canonical fallback is stored in an owned domain outcome.
        Self {
            value: "MCP request failed".to_owned(),
        }
    }

    /// Borrow the validated message text.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Display for RpcErrorMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Typed JSON-RPC error outcome produced by MCP routing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for RpcErrorBody."]
pub struct RpcErrorBody {
    code: RpcErrorCode,
    message: RpcErrorMessage,
}

impl RpcErrorBody {
    /// Construct a typed error from already validated domain values.
    pub const fn new(code: RpcErrorCode, message: RpcErrorMessage) -> Self {
        Self { code, message }
    }

    /// Error classification for transport encoding.
    pub const fn code(&self) -> RpcErrorCode {
        self.code
    }

    /// Validated human-readable error detail.
    pub fn message(&self) -> &RpcErrorMessage {
        &self.message
    }
}

/// A non-empty MCP tool identifier used by registry, alias, routing, and
/// stale-write-gate decisions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc = "Canonical domain representation for McpToolName."]
pub struct McpToolName {
    #[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
    value: String,
}

impl McpToolName {
    /// Validate a tool identifier received from an MCP boundary.
    ///
    /// Invalid or blank input is rejected before the identifier enters the domain.
    pub fn try_new(value: &str) -> Result<Self, DecodeError> {
        const MAX_LENGTH: usize = 64;
        let valid_characters = value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'));
        if value.is_empty() || value.len() > MAX_LENGTH || !valid_characters {
            return Err(DecodeError::new(
                "mcpToolName",
                "must be 1 to 64 ASCII letters, digits, underscores, dashes, dots, or slashes",
            ));
        }
        // ALLOC-JUSTIFICATION: the tool identity outlives the borrowed transport request.
        Ok(Self {
            value: value.to_owned(),
        })
    }

    /// Borrow the canonical identifier text for catalog lookup and output.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Display for McpToolName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Explicit write intent decoded from an MCP call's optional `write` field.
/// `Unspecified` preserves the wire distinction between absence and `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for McpWriteIntent."]
pub enum McpWriteIntent {
    Unspecified,
    Write,
    ReadOnly,
}

/// Server freshness supplied by the runtime fingerprint boundary before MCP
/// routing decides whether a coordination write is safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for McpFreshness."]
pub enum McpFreshness {
    Fresh,
    Stale,
    HashIncompatible,
}

/// Explicit execution preference decoded from an MCP call's optional
/// `dryRun` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for McpExecutionMode."]
pub enum McpExecutionMode {
    Unspecified,
    DryRun,
    Apply,
}

/// A non-empty action token carried by MCP tool arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for McpActionName."]
pub struct McpActionName {
    #[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
    value: String,
}

impl McpActionName {
    /// Validate an action identifier, rejecting invalid blank input.
    pub fn try_new(value: &str) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            return Err(DecodeError::new("mcpActionName", "must not be empty"));
        }
        // ALLOC-JUSTIFICATION: the action identity outlives the borrowed transport request.
        Ok(Self {
            value: value.to_owned(),
        })
    }

    #[doc = "The as_str operation for this canonical domain value."]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

/// Forward-slash rendering of an artifact location.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ArtifactPath."]
pub struct ArtifactPath {
    #[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
    rendered: String,
}

impl ArtifactPath {
    /// Capture a filesystem location in its canonical forward-slash form.
    pub fn from_path(path: &Path) -> Self {
        Self {
            rendered: path.to_string_lossy().replace('\\', "/"),
        }
    }

    /// View the canonical rendering as a filesystem path.
    pub fn as_path(&self) -> &Path {
        Path::new(&self.rendered)
    }

    /// Canonical rendered value for display or digest construction.
    pub fn as_str(&self) -> &str {
        &self.rendered
    }
}

/// A non-empty package version label.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for PackageVersion."]
pub struct PackageVersion {
    #[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
    semver: String,
}

impl PackageVersion {
    /// Validate and retain a package version value, rejecting invalid blank input.
    pub fn try_new(value: &str) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            return Err(DecodeError::new("packageVersion", "must not be empty"));
        }
        // ALLOC-JUSTIFICATION: the package version outlives the borrowed build metadata.
        Ok(Self {
            semver: value.to_owned(),
        })
    }

    /// Canonical version text for display or digest construction.
    pub fn as_str(&self) -> &str {
        &self.semver
    }
}

/// Exact byte length observed for a read artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: ByteCount privately owns an exact observed artifact length."]
pub struct ByteCount(u64);

impl ByteCount {
    /// Empty artifact length.
    pub const ZERO: Self = Self(0);

    /// Brand an already validated non-zero artifact length.
    pub const fn try_new(observed: std::num::NonZeroU64) -> Self {
        Self(observed.get())
    }
}

impl From<ByteCount> for u64 {
    fn from(value: ByteCount) -> Self {
        value.0
    }
}

/// Whether an artifact was read and hashed or could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ArtifactState."]
pub enum ArtifactState {
    Present {
        sha256: Sha256,
        byte_length: ByteCount,
    },
    Missing,
}

/// Fingerprint observation for one artifact location.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ArtifactEntry."]
pub struct ArtifactEntry {
    pub path: ArtifactPath,
    pub state: ArtifactState,
}

impl ArtifactEntry {
    /// Read and fingerprint a file, preserving an unreadable location as an
    /// explicit missing state.
    pub fn of_file(path: &Path) -> Self {
        let state = match std::fs::read(path) {
            Ok(bytes) => {
                // BRAND-INVARIANT: a checked platform length becomes ByteCount before storage.
                let byte_length = match u64::try_from(bytes.len()) {
                    Ok(observed) => std::num::NonZeroU64::new(observed)
                        .map_or(ByteCount::ZERO, ByteCount::try_new),
                    Err(_) => ByteCount::ZERO,
                };
                ArtifactState::Present {
                    sha256: crate::boundary::hash::validate(&bytes),
                    byte_length,
                }
            }
            Err(_) => ArtifactState::Missing,
        };
        Self {
            path: ArtifactPath::from_path(path),
            state,
        }
    }
}

impl std::fmt::Display for ArtifactEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.state {
            ArtifactState::Present {
                sha256,
                byte_length,
            } => write!(
                f,
                "{} present {} {}",
                self.path.as_str(),
                sha256,
                byte_length.0
            ),
            ArtifactState::Missing => write!(f, "{} missing", self.path.as_str()),
        }
    }
}

/// Combined fingerprint of a running MCP binary and optional ruleset.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for McpFingerprint."]
pub struct McpFingerprint {
    pub digest: Sha256,
    pub package_version: PackageVersion,
    pub binary: ArtifactEntry,
    pub ruleset: Option<ArtifactEntry>,
}

/// Failure to resolve the executable used for an MCP fingerprint.
#[derive(Debug, thiserror::Error)]
#[doc = "Canonical domain representation for FingerprintError."]
pub enum FingerprintError {
    #[error("could not resolve the running executable path: {source}")]
    CurrentExeUnresolvable {
        #[source]
        source: std::io::Error,
    },
}

/// Named artifact location inside an MCP fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for ArtifactSlot."]
pub enum ArtifactSlot {
    Binary,
    Ruleset,
}

/// A changed artifact between startup and current observations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for ChangedArtifact."]
pub struct ChangedArtifact {
    pub slot: ArtifactSlot,
    pub startup: Option<ArtifactEntry>,
    pub current: Option<ArtifactEntry>,
}

/// Freshness verdict for an MCP artifact fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical domain representation for Staleness."]
pub enum Staleness {
    Fresh,
    Stale,
}

/// Full fingerprint freshness comparison result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "Canonical domain representation for StalenessReport."]
pub struct StalenessReport {
    pub verdict: Staleness,
    pub startup_digest: Sha256,
    pub current_digest: Sha256,
    pub changed: Vec<ChangedArtifact>,
}

/// Build a fingerprint from explicit artifact locations and an already
/// validated build version.
pub fn build_mcp_fingerprint(
    binary_path: &Path,
    package_version: PackageVersion,
    ruleset_path: Option<&Path>,
) -> McpFingerprint {
    let binary = ArtifactEntry::of_file(binary_path);
    let ruleset = ruleset_path.map(ArtifactEntry::of_file);
    let digest = fold_digest(&binary, &package_version, ruleset.as_ref());
    McpFingerprint {
        digest,
        package_version,
        binary,
        ruleset,
    }
}

/// Compare a startup fingerprint with a fresh observation of the same
/// artifact locations.
pub fn compare_freshness(startup: &McpFingerprint, current: &McpFingerprint) -> StalenessReport {
    let verdict = if startup.digest == current.digest {
        Staleness::Fresh
    } else {
        Staleness::Stale
    };
    StalenessReport {
        // CLONE-JUSTIFICATION: this owned value crosses an independent result or record lifetime.
        verdict,
        // CLONE-JUSTIFICATION: this owned value crosses an independent result or record lifetime.
        startup_digest: startup.digest.clone(),
        current_digest: current.digest.clone(),
        changed: changed_slots(startup, current),
    }
}

impl McpFingerprint {
    /// Re-read the binary and optional ruleset locations captured at startup.
    pub fn recompute(&self) -> Self {
        let ruleset_path = self.ruleset.as_ref().map(|entry| entry.path.as_path());
        build_mcp_fingerprint(
            // CLONE-JUSTIFICATION: this owned value crosses an independent result or record lifetime.
            self.binary.path.as_path(),
            self.package_version.clone(),
            ruleset_path,
        )
    }

    /// Recompute from disk and return the typed freshness comparison.
    pub fn compare_to_current(&self) -> StalenessReport {
        let current = self.recompute();
        compare_freshness(self, &current)
    }
}

fn fold_digest(
    binary: &ArtifactEntry,
    package_version: &PackageVersion,
    ruleset: Option<&ArtifactEntry>,
) -> Sha256 {
    let ruleset_render = ruleset.map_or_else(|| String::from("untracked"), ToString::to_string);
    let preimage = format!(
        "enforcer-mcp-fingerprint-v1\nbinary={binary}\nversion={}\nruleset={ruleset_render}",
        package_version.as_str(),
    );
    crate::boundary::hash::validate(preimage.as_bytes())
}

fn changed_slots(startup: &McpFingerprint, current: &McpFingerprint) -> Vec<ChangedArtifact> {
    let mut changed = Vec::new();
    if startup.binary != current.binary {
        changed.push(ChangedArtifact {
            // CLONE-JUSTIFICATION: this owned value crosses an independent result or record lifetime.
            slot: ArtifactSlot::Binary,
            // CLONE-JUSTIFICATION: this owned value crosses an independent result or record lifetime.
            startup: Some(startup.binary.clone()),
            current: Some(current.binary.clone()),
        });
    }
    if startup.ruleset != current.ruleset {
        changed.push(ChangedArtifact {
            // CLONE-JUSTIFICATION: this owned value crosses an independent result or record lifetime.
            slot: ArtifactSlot::Ruleset,
            // CLONE-JUSTIFICATION: this owned value crosses an independent result or record lifetime.
            startup: startup.ruleset.clone(),
            current: current.ruleset.clone(),
        });
    }
    changed
}
