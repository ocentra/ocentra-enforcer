//! X06.8: signed memory bundles for sharing across
//! personal/team/community scopes.
//!
//! A bundle is a zstd-compressed archive of exactly one JSON payload
//! (the [`BundleManifest`] plus a [`BundleGraphSnapshot`]) plus a
//! DETACHED ed25519 signature over the COMPRESSED bytes -- signing the
//! compressed form (not the pre-compression JSON) means a verifier can
//! check the signature without first decompressing/trusting the payload
//! (see [`crate::federation`], which verifies signature and checksum
//! before touching the decompressed payload at all).
//!
//! # Scope and default
//!
//! [`Scope::Personal`] is this crate's DEFAULT scope. Every export --
//! including [`Scope::Personal`] -- REQUIRES [`ExportConsent::Granted`]
//! in the call: a bundle leaves the local machine the moment it exists
//! as a value the caller can write/transmit, so even a personal-scope
//! export must be an explicit, per-call opt-in. There is no ambient/
//! global consent setting this module reads from disk or the
//! environment.
//!
//! # D-11 -- the team graph bootstrap artifact
//!
//! The same bundle format IS the "compressed graph artifact" a team
//! bootstrap would use: a [`Scope::Team`] bundle whose
//! [`BundleGraphSnapshot`] carries the exporting project's full memory
//! graph (every record + lesson row) is exactly the artifact a new
//! teammate's `enforcer-memory` import would bootstrap from. There is
//! deliberately no second "graph artifact" format for records/lessons --
//! see [`crate::federation::import_bundle`]'s bootstrap-reconstruction
//! test. (The `.codebase-memory/graph.db.zst` code-graph artifact is a
//! SEPARATE format owned by [`crate::artifacts`] -- that one persists
//! [`crate::code_graph::CodeGraph`], not [`crate::graph::MemoryGraph`].)

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::graph::{MemoryGraph, MemoryNode};
use crate::lesson::LessonRow;
use crate::record::MemoryRecord;

/// Current wire schema version for [`BundleManifest`]/[`BundleGraphSnapshot`].
/// A bundle whose manifest carries a DIFFERENT version is rejected by
/// [`crate::federation::import_bundle`] rather than guessed at.
pub const BUNDLE_SCHEMA_VERSION: u32 = 1;

/// Sharing scope. [`Scope::Community`] additionally requires the payload
/// to have already been through [`crate::redaction::redact_record`] --
/// checked by [`export_bundle`] via the creator-field drop, not merely
/// documented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    /// Default. Stays on the exporting machine/user until the caller
    /// transmits it -- still requires [`ExportConsent::Granted`].
    Personal,
    /// Shared with a bounded team/org.
    Team,
    /// Shared publicly / with the broader community.
    Community,
}

/// Explicit, per-call consent to export. This is a value the caller
/// constructs and passes into [`export_bundle`] on every call -- there
/// is no persisted/ambient consent flag this module reads instead. Every
/// scope (including [`Scope::Personal`]) requires
/// [`ExportConsent::Granted`]: an export always produces a value that
/// can leave the local machine, so "personal" narrows WHO it is meant
/// for, not whether the caller had to opt in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportConsent {
    /// No consent given. The default value, so a caller that does not
    /// explicitly opt in gets this.
    #[default]
    NotGranted,
    /// Consent explicitly granted for this specific export call.
    Granted,
}

/// Export-time failures.
#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    /// Export was requested without [`ExportConsent::Granted`] in the
    /// same call, for any scope.
    #[error(
        "export at scope {scope:?} requires explicit consent in the call; consent was not granted"
    )]
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

/// The bundle manifest: everything a caller needs to know about a bundle
/// BEFORE trusting its content -- schema version, the git head the
/// exporting repo was at, a content hash of the (uncompressed) payload,
/// the scope, and the creator. Carried inside the bundle alongside the
/// payload (not just in the detached signature) so a caller inspecting
/// an already-verified bundle does not need to re-derive any of this.
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
    /// bundles (e.g. a writer/lane name). MUST be absent for
    /// [`Scope::Community`] bundles -- [`export_bundle`] enforces this by
    /// construction rather than trusting the caller to have redacted it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    pub created_at: String,
}

/// The exportable graph content: every `Record`/`Lesson` node from a
/// [`MemoryGraph`], flattened into the two wire-serializable shapes this
/// crate already defines. `Incident` nodes (raw local usage telemetry,
/// not shareable knowledge) are intentionally excluded.
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
/// and the detached signature over those compressed bytes.
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
    /// can look it up in the caller's trust list; the trust list (not
    /// this field's mere presence) is what makes the key trusted.
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
    if !raw.len().is_multiple_of(2) {
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
    pub scope: Scope,
    pub consent: ExportConsent,
    pub creator: Option<String>,
    pub git_head: Option<String>,
    pub created_at: String,
}

/// Export `graph` as a signed bundle per `request`. `request.consent`
/// must be [`ExportConsent::Granted`] for EVERY scope (checked before
/// any compression/signing work happens). `request.creator` is dropped
/// unconditionally for [`Scope::Community`] regardless of what the
/// caller passes, since a community bundle carries no personal creator
/// identity by construction (defense in depth alongside
/// [`crate::redaction::redact_record`], which the caller is still
/// responsible for having applied to the snapshot's records before
/// calling this for a community export).
pub fn export_bundle(
    snapshot: &BundleGraphSnapshot,
    request: ExportRequest,
    signing_key: &SigningKey,
) -> Result<SignedBundle, ShareError> {
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
        created_at,
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
/// standalone entry point.
pub(crate) fn decode_payload_unchecked(
    compressed_payload: &[u8],
) -> Result<(Vec<u8>, BundleGraphSnapshot), ShareError> {
    let decompressed = zstd::decode_all(compressed_payload).map_err(ShareError::Decompression)?;
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

    fn request(scope: Scope, consent: ExportConsent, creator: Option<String>) -> ExportRequest {
        ExportRequest {
            scope,
            consent,
            creator,
            git_head: None,
            created_at: "2026-07-05T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn personal_export_still_requires_consent() {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let outcome = export_bundle(
            &snapshot,
            request(
                Scope::Personal,
                ExportConsent::NotGranted,
                Some("primary".to_string()),
            ),
            &key,
        );
        assert!(matches!(
            outcome,
            Err(ShareError::ConsentRequired {
                scope: Scope::Personal
            })
        ));
    }

    #[test]
    fn personal_export_with_consent_succeeds() -> Result<(), ShareError> {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let bundle = export_bundle(
            &snapshot,
            request(
                Scope::Personal,
                ExportConsent::Granted,
                Some("primary".to_string()),
            ),
            &key,
        )?;
        assert_eq!(bundle.manifest.scope, Scope::Personal);
        assert_eq!(bundle.manifest.creator, Some("primary".to_string()));
        Ok(())
    }

    #[test]
    fn team_export_without_consent_is_rejected() {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let outcome = export_bundle(
            &snapshot,
            request(Scope::Team, ExportConsent::NotGranted, None),
            &key,
        );
        assert!(matches!(
            outcome,
            Err(ShareError::ConsentRequired { scope: Scope::Team })
        ));
    }

    #[test]
    fn team_export_with_consent_succeeds() -> Result<(), ShareError> {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let mut req = request(
            Scope::Team,
            ExportConsent::Granted,
            Some("team-lead".to_string()),
        );
        req.git_head = Some("abc123".to_string());
        let bundle = export_bundle(&snapshot, req, &key)?;
        assert_eq!(bundle.manifest.git_head, Some("abc123".to_string()));
        Ok(())
    }

    #[test]
    fn community_export_drops_creator_even_if_supplied() -> Result<(), ShareError> {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let bundle = export_bundle(
            &snapshot,
            request(
                Scope::Community,
                ExportConsent::Granted,
                Some("should-be-dropped".to_string()),
            ),
            &key,
        )?;
        assert!(bundle.manifest.creator.is_none());
        Ok(())
    }

    #[test]
    fn compressed_payload_round_trips_to_the_same_snapshot() -> Result<(), ShareError> {
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let bundle = export_bundle(
            &snapshot,
            request(Scope::Personal, ExportConsent::Granted, None),
            &key,
        )?;
        let (_, decoded) = decode_payload_unchecked(&bundle.compressed_payload)?;
        assert_eq!(decoded, snapshot);
        Ok(())
    }

    #[test]
    fn hex_round_trip() -> Result<(), String> {
        let bytes = vec![0u8, 1, 2, 255, 128, 17];
        let hex = hex_encode(&bytes);
        let back = hex_decode(&hex)?;
        assert_eq!(back, bytes);
        Ok(())
    }
}
