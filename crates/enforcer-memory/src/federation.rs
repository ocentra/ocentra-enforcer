//! X06.8: zero-trust bundle import.
//!
//! Three checks run, in a fixed order, before a bundle's content is
//! trusted enough to touch [`MemoryGraph`]: signature (against a local
//! [`TrustList`]) is checked FIRST, directly against the still-compressed
//! bytes -- so an untrusted or tampered bundle is rejected before this
//! module ever decompresses anything it did not sign for. Only once the
//! signature passes does [`import_bundle`] decompress the payload, then
//! verify the checksum (the manifest's recorded content hash against
//! those decompressed bytes) and finally the schema version. Any
//! failure produces a typed, reason-carrying rejection
//! ([`RejectedBundle`]) rather than a partially-applied import -- this
//! is the "zero-trust import" hard requirement: nothing from an
//! untrusted, tampered, or version-skewed bundle ever reaches the graph.
//!
//! # Activation
//!
//! Every record/lesson landing through [`import_bundle`] is forced
//! INACTIVE regardless of what the exporting repo's own `landedAt`
//! said: this repo has not run its own x05 local validation on imported
//! content yet, so [`crate::learning::lesson_status`] must report
//! [`enforcer_domain::memory_types::LessonStatus::Inactive`] for every imported id
//! until a local landing event activates it. Inactive is not hidden --
//! [`crate::recall::recall`] still finds imported content, per the
//! crate-wide "searchable but inactive" rule -- it is simply not counted
//! as proven yet.
//!
//! Activating an imported id is done through [`crate::learning`]'s
//! EXISTING supersede mechanism (not forked here): `lesson_status` keys
//! a single id's status off the FIRST node recorded under that id, so
//! this repo's own x05 validation lands a NEW record whose `supersedes`
//! names the imported id -- the new record is what
//! [`crate::learning::active_lessons`] reports as active, and
//! [`crate::learning::superseded_by`] keeps the audit trail from the
//! imported id to whatever locally validated it.

use crate::boundary::share::{
    decode_payload_unchecked, signature_for_bundle, verifying_key_for_bundle,
    BundleGraphSnapshotDto, SignedBundleDto, BUNDLE_SCHEMA_VERSION,
};
use crate::graph::MemoryGraph;
use crate::owned_boundary::{Retained, RetainedDisplay};
use enforcer_domain::memory_types::{
    MemoryBundleContentHash, MemoryBundlePayload, MemoryBundlePublicKeyHex,
    MemoryBundleSchemaVersion, MemoryFederationImportCount, MemoryFederationRejectedAt,
    MemoryTrustListMembership,
};

/// A local trust list: the set of ed25519 public keys (hex-encoded, as
/// carried in [`SignedBundleDto::signer_public_key_hex`]) this instance
/// accepts bundles from. Deliberately the simplest possible
/// representation (no expiry/revocation/rotation) -- callers needing
/// those layer them on top of `trusted_keys_hex`. This is a purely
/// local, injectable abstraction -- no network call, no PKI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustList {
    trusted_keys_hex: Vec<MemoryBundlePublicKeyHex>,
}

impl TrustList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Trust list containing exactly the given hex-encoded public keys.
    pub fn from_keys(keys_hex: impl IntoIterator<Item = MemoryBundlePublicKeyHex>) -> Self {
        Self {
            trusted_keys_hex: keys_hex.into_iter().collect(),
        }
    }

    pub fn trust(&mut self, key_hex: MemoryBundlePublicKeyHex) {
        self.trusted_keys_hex.push(key_hex);
    }

    pub fn is_trusted(&self, key_hex: &MemoryBundlePublicKeyHex) -> MemoryTrustListMembership {
        self.trusted_keys_hex
            .iter()
            .any(|trusted| trusted.as_str() == key_hex.as_str())
            .into()
    }
}

/// Why a bundle was rejected. Each variant names one of the three
/// zero-trust checks (or a fourth: the signing key itself being
/// unparseable, which is a signature-shaped failure) -- a caller
/// recording rejections never has to parse a message string to know
/// which gate failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// The signer's public key is not in the local [`TrustList`] (or the
    /// key/signature hex could not even be parsed, or the signature
    /// itself does not verify).
    UntrustedSigner(String),
    /// The manifest's recorded content hash does not match the actual
    /// decompressed payload bytes.
    ChecksumTamper { expected: String, actual: String },
    /// The manifest's `schema_version` is not
    /// [`crate::boundary::share::BUNDLE_SCHEMA_VERSION`].
    SchemaVersionIncompatible { found: u32, expected: u32 },
}

impl std::fmt::Display for RejectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RejectReason::UntrustedSigner(detail) => write!(f, "untrusted-signer: {detail}"),
            RejectReason::ChecksumTamper { expected, actual } => {
                write!(f, "checksum-tamper: expected {expected}, got {actual}")
            }
            RejectReason::SchemaVersionIncompatible { found, expected } => {
                write!(
                    f,
                    "schema-version-incompatible: found {found}, expected {expected}"
                )
            }
        }
    }
}

/// A bundle that failed zero-trust import, recorded with its reason
/// (never silently dropped). `at` is the caller-supplied timestamp of
/// the rejected import attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedBundle {
    pub reason: RejectReason,
    pub at: MemoryFederationRejectedAt,
}

/// The outcome of a successful import: how many records/lessons were
/// added to the graph, all forced inactive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub records_imported: MemoryFederationImportCount,
    pub lessons_imported: MemoryFederationImportCount,
}

impl ImportReport {
    pub fn total(&self) -> MemoryFederationImportCount {
        (self.records_imported.get() + self.lessons_imported.get()).into()
    }
}

/// An in-memory, append-only log of every rejected import attempt --
/// "rejections are recorded" per the zero-trust import contract. Callers
/// wire this alongside [`import_bundle`] (it is not threaded through
/// automatically, since import is a pure function over its inputs); see
/// [`import_bundle_logged`] for the convenience wrapper that does both in
/// one call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RejectionLog {
    entries: Vec<RejectedBundle>,
}

impl RejectionLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, rejected: RejectedBundle) {
        self.entries.push(rejected);
    }

    pub fn entries(&self) -> &[RejectedBundle] {
        &self.entries
    }
}

/// Verify `bundle` against `trust_list` (signature, then checksum, then
/// schema version) and, on success, ingest its snapshot into `graph`
/// with every record/lesson forced inactive (`landed_at` cleared).
/// Returns [`RejectedBundle`] on the FIRST check that fails -- checks
/// run in the fixed order signature -> checksum -> schema-version so a
/// caller triaging a rejection log always knows which gate to look at
/// first regardless of which OTHER checks would also have failed.
pub fn import_bundle(
    graph: &mut MemoryGraph,
    bundle: &SignedBundleDto,
    trust_list: &TrustList,
    at: &MemoryFederationRejectedAt,
) -> Result<ImportReport, RejectedBundle> {
    verify_signature(bundle, trust_list).map_err(|reason| RejectedBundle {
        reason,
        at: at.retained(),
    })?;

    let (decompressed, snapshot) =
        decode_payload_unchecked(&bundle.compressed_payload).map_err(|source| RejectedBundle {
            reason: RejectReason::ChecksumTamper {
                expected: bundle.manifest.content_hash.retained_display(),
                actual: format!("<undecodable: {source}>"),
            },
            at: at.retained(),
        })?;

    let decompressed: MemoryBundlePayload = decompressed.into();
    verify_checksum(&bundle.manifest.content_hash, &decompressed).map_err(|reason| {
        RejectedBundle {
            reason,
            at: at.retained(),
        }
    })?;

    verify_schema_version(bundle.manifest.schema_version).map_err(|reason| RejectedBundle {
        reason,
        at: at.retained(),
    })?;

    Ok(ingest_inactive(graph, snapshot))
}

/// Same as [`import_bundle`], but on rejection also appends the
/// [`RejectedBundle`] to `log` before returning it -- the convenience
/// entry point for callers that want every rejection recorded without
/// duplicating the `match`/`log.record(...)` boilerplate at every call
/// site.
pub fn import_bundle_logged(
    graph: &mut MemoryGraph,
    bundle: &SignedBundleDto,
    trust_list: &TrustList,
    at: &MemoryFederationRejectedAt,
    log: &mut RejectionLog,
) -> Result<ImportReport, RejectedBundle> {
    let outcome = import_bundle(graph, bundle, trust_list, at);
    if let Err(rejected) = &outcome {
        log.record(rejected.retained());
    }
    outcome
}

fn verify_signature(bundle: &SignedBundleDto, trust_list: &TrustList) -> Result<(), RejectReason> {
    if !trust_list
        .is_trusted(&bundle.signer_public_key_hex)
        .is_trusted()
    {
        return Err(RejectReason::UntrustedSigner(format!(
            "signer key {} is not in the local trust list",
            bundle.signer_public_key_hex
        )));
    }
    let verifying_key = verifying_key_for_bundle(&bundle.signer_public_key_hex)
        .ok_or_else(|| RejectReason::UntrustedSigner("unparseable public key".retained()))?;
    let signature = signature_for_bundle(&bundle.signature_hex)
        .ok_or_else(|| RejectReason::UntrustedSigner("unparseable signature".retained()))?;
    verifying_key
        .verify_strict(&bundle.compressed_payload, &signature)
        .map_err(|source| RejectReason::UntrustedSigner(format!("verification failed: {source}")))
}

fn verify_checksum(
    expected_hash: &MemoryBundleContentHash,
    decompressed_payload: &MemoryBundlePayload,
) -> Result<(), RejectReason> {
    let actual = enforcer_core::hash_chain::link_digest(None, decompressed_payload.as_ref());
    if actual.as_str() != expected_hash.as_str() {
        return Err(RejectReason::ChecksumTamper {
            expected: expected_hash.as_str().into(),
            actual: actual.retained_display(),
        });
    }
    Ok(())
}

fn verify_schema_version(found: MemoryBundleSchemaVersion) -> Result<(), RejectReason> {
    if u32::from(found) != BUNDLE_SCHEMA_VERSION {
        return Err(RejectReason::SchemaVersionIncompatible {
            found: found.into(),
            expected: BUNDLE_SCHEMA_VERSION,
        });
    }
    Ok(())
}

/// Ingest every record/lesson in `snapshot` into `graph`, forcing each
/// one inactive (`landed_at` cleared) regardless of what the exporting
/// repo recorded.
fn ingest_inactive(graph: &mut MemoryGraph, snapshot: BundleGraphSnapshotDto) -> ImportReport {
    let mut report = ImportReport::default();
    for mut dto in snapshot.records {
        dto.landed_at.clear();
        graph.ingest_record(crate::record::MemoryRecord::from_dto(dto));
        report.records_imported += 1;
    }
    for mut lesson in snapshot.lessons {
        lesson.landed_at.clear();
        graph.ingest_lesson_row(lesson.into());
        report.lessons_imported += 1;
    }
    report
}
