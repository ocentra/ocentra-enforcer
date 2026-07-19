//! X06.8: signed memory bundles for sharing across
//! personal/team/community scopes.
//!
//! A bundle is a zstd-compressed archive of exactly one JSON payload
//! (the [`BundleManifestDto`] plus a [`BundleGraphSnapshotDto`]) plus a
//! DETACHED ed25519 signature over the COMPRESSED bytes -- signing the
//! compressed form (not the pre-compression JSON) means a verifier can
//! check the signature without first decompressing/trusting the payload
//! (see [`crate::federation`], which verifies signature and checksum
//! before touching the decompressed payload at all).
//!
//! # MemoryShareScope and default
//!
//! [`MemoryShareScope::Personal`] is this crate's DEFAULT scope. Every export --
//! including [`MemoryShareScope::Personal`] -- REQUIRES [`ExportConsent::Granted`]
//! in the call: a bundle leaves the local machine the moment it exists
//! as a value the caller can write/transmit, so even a personal-scope
//! export must be an explicit, per-call opt-in. There is no ambient/
//! global consent setting this module reads from disk or the
//! environment.
//!
//! # D-11 -- the team graph bootstrap artifact
//!
//! The same bundle format IS the "compressed graph artifact" a team
//! bootstrap would use: a [`MemoryShareScope::Team`] bundle whose
//! [`BundleGraphSnapshotDto`] carries the exporting project's full memory
//! graph (every record + lesson row) is exactly the artifact a new
//! teammate's `enforcer-memory` import would bootstrap from. There is
//! deliberately no second "graph artifact" format for records/lessons --
//! see [`crate::federation::import_bundle`]'s bootstrap-reconstruction
//! test. (The `.codebase-memory/graph.db.zst` code-graph artifact is a
//! SEPARATE format owned by [`crate::artifacts`] -- that one persists
//! [`crate::code_graph::CodeGraph`], not [`crate::graph::MemoryGraph`].)

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use enforcer_domain::memory_types::{
    ExportConsent, MemoryBundleContentHash, MemoryBundleCreatedAt, MemoryBundleCreator,
    MemoryBundleGitHead, MemoryBundleNodeCount, MemoryBundlePayload, MemoryBundlePublicKeyHex,
    MemoryBundleSchemaVersion, MemoryBundleSignatureHex, MemoryLedgerLessonId, MemoryLessonDate,
    MemoryLessonLandedAt, MemoryLessonObserved, MemoryLessonShipsVia, MemoryLessonText,
    MemoryShareScope,
};
use serde::{Deserialize, Serialize};

use crate::boundary::record::MemoryRecordDto;
use crate::graph::{MemoryGraph, MemoryNode};
use crate::lesson::LessonRow;
use crate::owned_boundary::RetainedDisplay;

/// Current wire schema version for [`BundleManifestDto`]/[`BundleGraphSnapshotDto`].
/// A bundle whose manifest carries a DIFFERENT version is rejected by
/// [`crate::federation::import_bundle`] rather than guessed at.
pub const BUNDLE_SCHEMA_VERSION: u32 = 1;

// ROUNDTRIP-TEST: unit_share::export_and_verify_personal_bundle_roundtrips
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonRowDto {
    pub id: MemoryLedgerLessonId,
    pub date: MemoryLessonDate,
    pub observed: MemoryLessonObserved,
    pub lesson: MemoryLessonText,
    pub landed_at: MemoryLessonLandedAt,
    pub ships_via: MemoryLessonShipsVia,
}

impl From<&LessonRow> for LessonRowDto {
    fn from(row: &LessonRow) -> Self {
        Self {
            id: row.id.clone(),
            date: row.date.clone(),
            observed: row.observed.clone(),
            lesson: row.lesson.clone(),
            landed_at: row.landed_at.clone(),
            ships_via: row.ships_via.clone(),
        }
    }
}

impl From<LessonRowDto> for LessonRow {
    fn from(row: LessonRowDto) -> Self {
        Self {
            id: row.id,
            date: row.date,
            observed: row.observed,
            lesson: row.lesson,
            landed_at: row.landed_at,
            ships_via: row.ships_via,
        }
    }
}

/// Sharing scope. [`MemoryShareScope::Community`] additionally requires the payload
/// to have already been through [`crate::redaction::redact_record`] --
/// checked by [`export_bundle`] via the creator-field drop, not merely
/// documented.
/// Explicit, per-call consent to export. This is a value the caller
/// constructs and passes into [`export_bundle`] on every call -- there
/// is no persisted/ambient consent flag this module reads instead. Every
/// scope (including [`MemoryShareScope::Personal`]) requires
/// [`ExportConsent::Granted`]: an export always produces a value that
/// can leave the local machine, so "personal" narrows WHO it is meant
/// for, not whether the caller had to opt in.
/// Export-time failures.
#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    /// Export was requested without [`ExportConsent::Granted`] in the
    /// same call, for any scope.
    #[error(
        "export at scope {scope:?} requires explicit consent in the call; consent was not granted"
    )]
    ConsentRequired { scope: MemoryShareScope },
    /// zstd compression of the manifest+payload JSON failed.
    #[error("bundle compression failed: {0}")]
    Compression(#[source] std::io::Error),
    /// zstd decompression of a bundle's archive bytes failed.
    #[error("bundle decompression failed: {0}")]
    Decompression(#[source] std::io::Error),
    /// JSON (de)serialization of the manifest+payload failed.
    #[error("bundle json codec failed: {0}")]
    Json(#[from] serde_json::Error),
}

/// The bundle manifest: everything a caller needs to know about a bundle
/// BEFORE trusting its content -- schema version, the git head the
/// exporting repo was at, a content hash of the (uncompressed) payload,
/// the scope, and the creator. Carried inside the bundle alongside the
/// payload (not just in the detached signature) so a caller inspecting
/// an already-verified bundle does not need to re-derive any of this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifestDto {
    pub schema_version: MemoryBundleSchemaVersion,
    /// The exporting repo's HEAD commit SHA, if known
    /// ([`crate::git::GitMetadata::head_commit`]). `None` for a
    /// synthetic/test export with no git context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_head: Option<MemoryBundleGitHead>,
    /// Content hash (`sha256:<64 hex>`) of the UNCOMPRESSED
    /// [`BundleGraphSnapshotDto`] JSON payload -- this is the checksum
    /// [`crate::federation::import_bundle`] re-verifies after
    /// decompression, independent of the ed25519 signature (which covers
    /// the compressed bytes as a whole).
    pub content_hash: MemoryBundleContentHash,
    pub scope: MemoryShareScope,
    /// Free-text creator identity for [`MemoryShareScope::Personal`]/[`MemoryShareScope::Team`]
    /// bundles (e.g. a writer/lane name). MUST be absent for
    /// [`MemoryShareScope::Community`] bundles -- [`export_bundle`] enforces this by
    /// construction rather than trusting the caller to have redacted it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<MemoryBundleCreator>,
    pub created_at: MemoryBundleCreatedAt,
}

/// The exportable graph content: every `Record`/`Lesson` node from a
/// [`MemoryGraph`], flattened into the two wire-serializable shapes this
/// crate already defines. `Incident` nodes (raw local usage telemetry,
/// not shareable knowledge) are intentionally excluded.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGraphSnapshotDto {
    pub records: Vec<MemoryRecordDto>,
    pub lessons: Vec<LessonRowDto>,
}

impl BundleGraphSnapshotDto {
    /// Snapshot every `Record`/`Lesson` node currently in `graph`.
    pub fn from_graph(graph: &MemoryGraph) -> Self {
        let mut records = Vec::new();
        let mut lessons = Vec::new();
        for node in graph.nodes() {
            match node {
                MemoryNode::Record(record) => records.push(record.to_dto()),
                MemoryNode::Lesson(row) => lessons.push(row.into()),
                MemoryNode::Incident(_) => {}
            }
        }
        Self { records, lessons }
    }

    /// Total node count this snapshot would reconstruct -- the figure
    /// [`crate::federation::import_bundle`]'s round-trip test compares
    /// against the rebuilt graph's `len()`.
    pub fn node_count(&self) -> MemoryBundleNodeCount {
        (self.records.len() + self.lessons.len()).into()
    }
}

/// A fully-assembled bundle: the manifest, the compressed payload bytes,
/// and the detached signature over those compressed bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedBundleDto {
    pub manifest: BundleManifestDto,
    /// zstd-compressed JSON bytes of the [`BundleGraphSnapshotDto`].
    #[serde(with = "hex_bytes")]
    pub compressed_payload: MemoryBundlePayload,
    /// Detached ed25519 signature over `compressed_payload`, hex-encoded.
    pub signature_hex: MemoryBundleSignatureHex,
    /// Hex-encoded ed25519 public key the signature verifies against --
    /// carried alongside the bundle so [`crate::federation::import_bundle`]
    /// can look it up in the caller's trust list; the trust list (not
    /// this field's mere presence) is what makes the key trusted.
    pub signer_public_key_hex: MemoryBundlePublicKeyHex,
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{hex_decode, hex_encode, MemoryBundlePayload};

    pub fn serialize<S: Serializer>(
        bytes: &MemoryBundlePayload,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        hex_encode(bytes.as_slice()).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<MemoryBundlePayload, D::Error> {
        let raw = String::deserialize(deserializer)?;
        hex_decode(&raw)
            .map(Into::into)
            .map_err(serde::de::Error::custom)
    }
}

/// Minimal hex encode -- no extra crate dependency for a byte vector
/// that is at most a few hundred KB of compressed payload.
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Minimal hex decode, fail-closed on odd length or non-hex chars.
pub(crate) fn hex_decode(raw: &str) -> Result<Vec<u8>, String> {
    if raw.len() % 2 != 0 {
        return Err(format!("odd-length hex string ({} chars)", raw.len()));
    }
    (0..raw.len())
        .step_by(2)
        .map(|i| {
            raw.get(i..i + 2)
                .ok_or_else(|| format!("truncated hex at offset {i}"))
                .and_then(|byte_str| {
                    u8::from_str_radix(byte_str, 16)
                        .map_err(|source| format!("invalid hex byte at offset {i}: {source}"))
                })
        })
        .collect()
}

/// Bundled arguments for [`export_bundle`] -- grouped into one struct
/// (rather than passed positionally) both to keep the function under
/// clippy's `too_many_arguments` threshold and so a future field (e.g. a
/// bundle description) does not require touching every call site's
/// argument order.
#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub scope: MemoryShareScope,
    pub consent: ExportConsent,
    pub creator: Option<MemoryBundleCreator>,
    pub git_head: Option<MemoryBundleGitHead>,
    pub created_at: MemoryBundleCreatedAt,
}

/// Export `graph` as a signed bundle per `request`. `request.consent`
/// must be [`ExportConsent::Granted`] for EVERY scope (checked before
/// any compression/signing work happens). `request.creator` is dropped
/// unconditionally for [`MemoryShareScope::Community`] regardless of what the
/// caller passes, since a community bundle carries no personal creator
/// identity by construction (defense in depth alongside
/// [`crate::redaction::redact_record`], which the caller is still
/// responsible for having applied to the snapshot's records before
/// calling this for a community export).
pub fn export_bundle(
    snapshot: &BundleGraphSnapshotDto,
    request: ExportRequest,
    signing_key: &SigningKey,
) -> Result<SignedBundleDto, ShareError> {
    let ExportRequest {
        scope,
        consent,
        creator,
        git_head,
        created_at,
    } = request;

    if consent != ExportConsent::Granted {
        return Err(ShareError::ConsentRequired { scope });
    }

    let payload_json = serde_json::to_vec(snapshot)?;
    let content_hash = enforcer_core::hash_chain::link_digest(None, &payload_json);

    let compressed_payload =
        zstd::encode_all(payload_json.as_slice(), 0).map_err(ShareError::Compression)?;

    let manifest = BundleManifestDto {
        schema_version: BUNDLE_SCHEMA_VERSION.into(),
        git_head,
        content_hash: content_hash.retained_display().into(),
        scope,
        creator: if matches!(scope, MemoryShareScope::Community) {
            None
        } else {
            creator
        },
        created_at,
    };

    let signature: Signature = signing_key.sign(&compressed_payload);

    Ok(SignedBundleDto {
        manifest,
        compressed_payload: compressed_payload.into(),
        signature_hex: hex_encode(&signature.to_bytes()).into(),
        signer_public_key_hex: hex_encode(signing_key.verifying_key().as_bytes()).into(),
    })
}

/// Decompress and parse a bundle's payload WITHOUT verifying its
/// signature or checksum -- an internal helper for
/// [`crate::federation::import_bundle`], which performs those checks
/// first and only calls this after they pass. Exposed at crate
/// visibility (not `pub`) so it can never be mistaken for a safe
/// standalone entry point.
pub(crate) fn decode_payload_unchecked(
    compressed_payload: &[u8],
) -> Result<(Vec<u8>, BundleGraphSnapshotDto), ShareError> {
    let decompressed = zstd::decode_all(compressed_payload).map_err(ShareError::Decompression)?;
    let snapshot: BundleGraphSnapshotDto = serde_json::from_slice(&decompressed)?;
    Ok((decompressed, snapshot))
}

/// Parse a hex-encoded ed25519 public key (as carried in
/// [`SignedBundleDto::signer_public_key_hex`]) into a [`VerifyingKey`].
pub(crate) fn parse_verifying_key(hex: &str) -> Option<VerifyingKey> {
    let bytes = hex_decode(hex).ok()?;
    let array: [u8; 32] = bytes.try_into().ok()?;
    VerifyingKey::from_bytes(&array).ok()
}

/// Decode a bundle signer key at the wire boundary before federation
/// verification consumes it.
pub(crate) fn verifying_key_for_bundle(key: &MemoryBundlePublicKeyHex) -> Option<VerifyingKey> {
    parse_verifying_key(key.as_str())
}

/// Parse a hex-encoded detached signature (as carried in
/// [`SignedBundleDto::signature_hex`]) into a [`Signature`].
pub(crate) fn parse_signature(hex: &str) -> Option<Signature> {
    let bytes = hex_decode(hex).ok()?;
    let array: [u8; 64] = bytes.try_into().ok()?;
    Some(Signature::from_bytes(&array))
}

/// Decode a bundle signature at the wire boundary before federation
/// verification consumes it.
pub(crate) fn signature_for_bundle(signature_hex: &MemoryBundleSignatureHex) -> Option<Signature> {
    parse_signature(signature_hex.as_str())
}
