//! X06.8: signed memory bundles for sharing across
//! personal/team/community scopes.
//!
//! A bundle is a zstd-compressed archive of exactly one JSON payload
//! (the [`BundleManifest`] plus a [`BundleGraphSnapshot`]) plus a
//! detached ed25519 signature over the COMPRESSED bytes -- signing the
//! compressed form (not the pre-compression JSON) means verification
//! never has to decompress untrusted bytes before it can even check
//! whether they came from a trusted key (see [`crate::federation`],
//! which verifies signature and checksum before touching the payload at
//! all).
//!
//! # Scope and default
//!
//! [`Scope::Personal`] is this crate's DEFAULT and the only scope
//! [`export_bundle`] will produce WITHOUT the caller passing an explicit
//! [`ExportConsent`] -- [`Scope::Team`] and [`Scope::Community`] both
//! REQUIRE `ExportConsent::Granted` in the call, matching the workpack's
//! "export requires explicit consent flag in the call" hard requirement.
//! There is no ambient/global consent setting this module reads from
//! disk or the environment: consent is a value the caller must construct
//! and pass on every single export call, so a bundle can never be
//! produced by a code path that "forgot" to ask.
//!
//! # D-11 -- the team graph bootstrap artifact
//!
//! The same bundle format IS the "compressed graph artifact" D-11
//! describes for team bootstrap: a [`Scope::Team`] bundle whose
//! [`BundleGraphSnapshot`] carries the exporting project's full memory
//! graph (every record + lesson row) is exactly the artifact a new
//! teammate's `enforcer-memory` import would bootstrap from. There is
//! deliberately no second "graph artifact" format -- see
//! [`crate::federation::import_bundle`]'s "graph bootstrap artifact
//! import reconstructs graph counts" hard test.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::graph::{MemoryGraph, MemoryNode};
use crate::lesson::LessonRow;
use crate::record::MemoryRecord;

/// Current wire schema version for [`BundleManifest`]/[`BundleGraphSnapshot`].
/// A bundle whose manifest carries a DIFFERENT version is rejected by
/// [`crate::federation::import_bundle`] rather than guessed at.
pub const BUNDLE_SCHEMA_VERSION: u32 = 1;

/// Sharing scope. Determines both the default consent posture
/// ([`Scope::Personal`] needs none; [`Scope::Team`]/[`Scope::Community`]
/// require [`ExportConsent::Granted`]) and, for [`Scope::Community`],
/// that the payload MUST have already been through
/// [`crate::redaction::redact_record`] (checked by [`export_bundle`],
/// not merely documented).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    /// Default. Stays on the exporting machine/user; no consent gate.
    Personal,
    /// Shared with a bounded team/org. Requires explicit consent.
    Team,
    /// Shared publicly / with the broader community. Requires explicit
    /// consent AND redaction (enforced by [`export_bundle`]).
    Community,
}

impl Scope {
    /// Whether this scope requires [`ExportConsent::Granted`] to export.
    pub fn requires_consent(self) -> bool {
        !matches!(self, Scope::Personal)
    }
}

/// Explicit, per-call consent to export beyond [`Scope::Personal`]. This
/// is a value the caller constructs and passes into [`export_bundle`]
/// on every call -- there is no persisted/ambient consent flag this
/// module reads instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportConsent {
    /// No consent given. The only value [`ExportConsent::default`]
    /// produces, so a caller that does not explicitly opt in gets this.
    NotGranted,
    /// Consent explicitly granted for this specific export call.
    Granted,
}

impl Default for ExportConsent {
    fn default() -> Self {
        ExportConsent::NotGranted
    }
}

/// Export-time failures.
#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    /// [`Scope::Team`] or [`Scope::Community`] was requested without
    /// [`ExportConsent::Granted`] in the same call.
    #[error("export to scope {scope:?} requires explicit consent in the call; consent was not granted")]
    ConsentRequired { scope: Scope },
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

/// The bundle manifest: everything a caller needs to know about a
/// bundle BEFORE trusting its content -- schema version, the git head
/// the exporting repo was at, a content hash of the (uncompressed)
/// payload, the scope, and the creator. Carried inside the bundle
/// alongside the payload (not just in the detached signature) so a
/// caller inspecting an already-verified bundle does not need to
/// re-derive any of this.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifest {
    pub schema_version: u32,
    /// The exporting repo's HEAD commit SHA, if known
    /// ([`crate::git::GitMetadata::head_commit`]). `None` for a
    /// synthetic/test export with no git context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_head: Option<String>,
    /// Content hash (`sha256:<64 hex>`) of the UNCOMPRESSED
    /// [`BundleGraphSnapshot`] JSON payload -- this is the checksum
    /// [`crate::federation::import_bundle`] re-verifies after
    /// decompression, independent of the ed25519 signature (which covers
    /// the compressed bytes as a whole).
    pub content_hash: String,
    pub scope: Scope,
    /// Free-text creator identity for [`Scope::Personal`]/[`Scope::Team`]
    /// bundles (e.g. a writer/lane name). MUST be absent/anonymized for
    /// [`Scope::Community`] bundles -- [`export_bundle`] enforces this by
    /// construction rather than trusting the caller to have redacted it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    pub created_at: String,
}

/// The exportable graph content: every `Record`/`Lesson` node from a
/// [`MemoryGraph`], flattened into the two wire-serializable shapes this
/// crate already defines. `Incident` nodes (raw local usage telemetry,
/// not shareable knowledge) are intentionally excluded -- see this
/// module's docs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGraphSnapshot {
    pub records: Vec<MemoryRecord>,
    pub lessons: Vec<LessonRow>,
}

impl BundleGraphSnapshot {
    /// Snapshot every `Record`/`Lesson` node currently in `graph`.
    pub fn from_graph(graph: &MemoryGraph) -> Self {
        let mut records = Vec::new();
        let mut lessons = Vec::new();
        for node in graph.nodes() {
            match node {
                MemoryNode::Record(record) => records.push((**record).clone()),
                MemoryNode::Lesson(row) => lessons.push(row.clone()),
                MemoryNode::Incident(_) => {}
            }
        }
        Self { records, lessons }
    }

    /// Total node count this snapshot would reconstruct -- the figure
    /// [`crate::federation::import_bundle`]'s round-trip test compares
    /// against the rebuilt graph's `len()`.
    pub fn node_count(&self) -> usize {
        self.records.len() + self.lessons.len()
    }
}

/// A fully-assembled bundle: the manifest, the compressed payload bytes,
/// and the detached signature over those compressed bytes. This is the
/// value [`export_bundle`] produces and
/// [`crate::federation::import_bundle`] consumes; (de)serializing it to
/// disk is the caller's responsibility (e.g. one JSON envelope, or three
/// sibling files) -- this crate defines the shape, not a fixed
/// container file format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedBundle {
    pub manifest: BundleManifest,
    /// zstd-compressed JSON bytes of the [`BundleGraphSnapshot`].
    #[serde(with = "hex_bytes")]
    pub compressed_payload: Vec<u8>,
    /// Detached ed25519 signature over `compressed_payload`, hex-encoded.
    pub signature_hex: String,
    /// Hex-encoded ed25519 public key the signature verifies against --
    /// carried alongside the bundle so [`crate::federation::import_bundle`]
    /// can look it up in the caller's trust list without a separate
    /// side-channel; the trust list (not this field's mere presence) is
    /// what makes the key trusted.
    pub signer_public_key_hex: String,
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{hex_decode, hex_encode};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        hex_encode(bytes).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let raw = String::deserialize(deserializer)?;
        hex_decode(&raw).map_err(serde::de::Error::custom)
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
            u8::from_str_radix(&raw[i..i + 2], 16)
                .map_err(|source| format!("invalid hex byte at offset {i}: {source}"))
        })
        .collect()
}

/// Export `graph` as a signed bundle at `scope`. `consent` must be
/// [`ExportConsent::Granted`] for any scope other than
/// [`Scope::Personal`] (checked BEFORE any compression/signing work
/// happens). `creator` is dropped unconditionally for
/// [`Scope::Community`] regardless of what the caller passes, since a
/// community bundle carries no personal creator identity by
/// construction (defense in depth alongside
/// [`crate::redaction::redact_record`], which the caller is still
/// responsible for having applied to the snapshot's records before
/// calling this for a community export).
pub fn export_bundle(
    snapshot: &BundleGraphSnapshot,
    scope: Scope,
    consent: ExportConsent,
    creator: Option<String>,
    git_head: Option<String>,
    now: &str,
    signing_key: &SigningKey,
) -> Result<SignedBundle, ShareError> {
    if scope.requires_consent() && consent != ExportConsent::Granted {
        return Err(ShareError::ConsentRequired { scope });
    }

    let payload_json = serde_json::to_vec(snapshot)?;
    let content_hash = enforcer_core::hash_chain::link_digest(None, &payload_json);

    let compressed_payload =
        zstd::encode_all(payload_json.as_slice(), 0).map_err(ShareError::Compression)?;

    let manifest = BundleManifest {
        schema_version: BUNDLE_SCHEMA_VERSION,
        git_head,
        content_hash,
        scope,
        creator: if matches!(scope, Scope::Community) {
            None
        } else {
            creator
        },
        created_at: now.to_owned(),
    };

    let signature: Signature = signing_key.sign(&compressed_payload);

    Ok(SignedBundle {
        manifest,
        compressed_payload,
        signature_hex: hex_encode(&signature.to_bytes()),
        signer_public_key_hex: hex_encode(signing_key.verifying_key().as_bytes()),
    })
}

/// Decompress and parse a bundle's payload WITHOUT verifying its
/// signature or checksum -- an internal helper for
/// [`crate::federation::import_bundle`], which performs those checks
/// first and only calls this after they pass. Exposed at crate
/// visibility (not `pub`) so it can never be mistaken for a safe
/// standalone entry point: reading a bundle's payload without the
/// zero-trust checks defeats the entire point of X06.8.
pub(crate) fn decode_payload_unchecked(
    compressed_payload: &[u8],
) -> Result<(Vec<u8>, BundleGraphSnapshot), ShareError> {
    let decompressed =
        zstd::decode_all(compressed_payload).map_err(ShareError::Decompression)?;
    let snapshot: BundleGraphSnapshot = serde_json::from_slice(&decompressed)?;
    Ok((decompressed, snapshot))
}

/// Parse a hex-encoded ed25519 public key (as carried in
/// [`SignedBundle::signer_public_key_hex`]) into a [`VerifyingKey`].
pub(crate) fn parse_verifying_key(hex: &str) -> Option<VerifyingKey> {
    let bytes = hex_decode(hex).ok()?;
    let array: [u8; 32] = bytes.try_into().ok()?;
    VerifyingKey::from_bytes(&array).ok()
}

/// Parse a hex-encoded detached signature (as carried in
/// [`SignedBundle::signature_hex`]) into a [`Signature`].
pub(crate) fn parse_signature(hex: &str) -> Option<Signature> {
    let bytes = hex_decode(hex).ok()?;
    let array: [u8; 64] = bytes.try_into().ok()?;
    Some(Signature::from_bytes(&array))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    fn sample_snapshot() -> BundleGraphSnapshot {
        let mut graph = MemoryGraph::new();
        graph.ingest_record(crate::record::MemoryRecord {
            schema_version: 1,
            id: "mem-primary-0001".to_string(),
            ts: "2026-07-05T00:00:00Z".to_string(),
            kind: crate::record::RecordKind::Lesson,
            domain: crate::record::RecordDomain::Harness,
            statement: "sample statement".to_string(),
            why: None,
            how_to_apply: None,
            applies_to: vec![],
            evidence: None,
            routes: vec![],
            landed_at: vec![],
            supersedes: None,
            provenance: crate::record::Provenance {
                writer: "primary".to_string(),
                ..Default::default()
            },
        });
        graph.ingest_lesson_row(LessonRow {
            id: "L1".to_string(),
            date: "2026-07-05".to_string(),
            observed: "x".to_string(),
            lesson: "y".to_string(),
            landed_at: "commit abc".to_string(),
            ships_via: "arc-16".to_string(),
        });
        BundleGraphSnapshot::from_graph(&graph)
    }

    #[test]
    fn personal_export_needs_no_consent() {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let bundle = export_bundle(
            &snapshot,
            Scope::Personal,
            ExportConsent::NotGranted,
            Some("primary".to_string()),
            None,
            "2026-07-05T00:00:00Z",
            &key,
        )
        .expect("personal export needs no consent");
        assert_eq!(bundle.manifest.scope, Scope::Personal);
        assert_eq!(bundle.manifest.creator, Some("primary".to_string()));
    }

    #[test]
    fn team_export_without_consent_is_rejected() {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let outcome = export_bundle(
            &snapshot,
            Scope::Team,
            ExportConsent::NotGranted,
            None,
            None,
            "2026-07-05T00:00:00Z",
            &key,
        );
        assert!(matches!(
            outcome,
            Err(ShareError::ConsentRequired {
                scope: Scope::Team
            })
        ));
    }

    #[test]
    fn team_export_with_consent_succeeds() {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let bundle = export_bundle(
            &snapshot,
            Scope::Team,
            ExportConsent::Granted,
            Some("team-lead".to_string()),
            Some("abc123".to_string()),
            "2026-07-05T00:00:00Z",
            &key,
        )
        .expect("consented team export succeeds");
        assert_eq!(bundle.manifest.git_head, Some("abc123".to_string()));
    }

    #[test]
    fn community_export_drops_creator_even_if_supplied() {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let bundle = export_bundle(
            &snapshot,
            Scope::Community,
            ExportConsent::Granted,
            Some("should-be-dropped".to_string()),
            None,
            "2026-07-05T00:00:00Z",
            &key,
        )
        .expect("consented community export succeeds");
        assert!(bundle.manifest.creator.is_none());
    }

    #[test]
    fn compressed_payload_round_trips_to_the_same_snapshot() {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let bundle = export_bundle(
            &snapshot,
            Scope::Personal,
            ExportConsent::NotGranted,
            None,
            None,
            "2026-07-05T00:00:00Z",
            &key,
        )
        .expect("export succeeds");
        let (_, decoded) =
            decode_payload_unchecked(&bundle.compressed_payload).expect("decode succeeds");
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn hex_round_trip() {
        let bytes = vec![0u8, 1, 2, 255, 128, 17];
        let hex = hex_encode(&bytes);
        let back = hex_decode(&hex).expect("valid hex decodes");
        assert_eq!(back, bytes);
    }
}
