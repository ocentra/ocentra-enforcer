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
//! [`crate::learning::LessonStatus::Inactive`] for every imported id
//! until a local landing event activates it. Inactive is not hidden --
//! [`crate::recall::recall`] still finds imported content, per the
//! crate-wide "searchable but inactive" rule -- it is simply not counted
//! as proven yet.
//!
//! Activating an imported id is done through
//! [`crate::learning`]'s EXISTING supersede mechanism (X06.6, not owned
//! by this subpack), not by re-ingesting the same id: `lesson_status`
//! keys a single id's status off the FIRST node recorded under that id,
//! so this repo's own x05 validation lands a NEW record whose
//! `supersedes` names the imported id -- the new record is what
//! [`crate::learning::active_lessons`] reports as active, and
//! [`crate::learning::superseded_by`] keeps the audit trail from the
//! imported id to whatever locally validated it.

use crate::graph::MemoryGraph;
use crate::share::{
    decode_payload_unchecked, parse_signature, parse_verifying_key, BundleGraphSnapshot,
    SignedBundle, BUNDLE_SCHEMA_VERSION,
};

/// A local trust list: the set of ed25519 public keys (hex-encoded, as
/// carried in [`SignedBundle::signer_public_key_hex`]) this instance
/// accepts bundles from. Deliberately the simplest possible
/// representation (no expiry/revocation/rotation) -- callers needing
/// those layer them on top of `trusted_keys_hex`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustList {
    trusted_keys_hex: Vec<String>,
}

impl TrustList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Trust list containing exactly the given hex-encoded public keys.
    pub fn from_keys(keys_hex: impl IntoIterator<Item = String>) -> Self {
        Self {
            trusted_keys_hex: keys_hex.into_iter().collect(),
        }
    }

    pub fn trust(&mut self, key_hex: impl Into<String>) {
        self.trusted_keys_hex.push(key_hex.into());
    }

    pub fn is_trusted(&self, key_hex: &str) -> bool {
        self.trusted_keys_hex.iter().any(|k| k == key_hex)
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
    /// key/signature hex could not even be parsed).
    Signature(String),
    /// The manifest's recorded content hash does not match the actual
    /// decompressed payload bytes.
    Checksum { expected: String, actual: String },
    /// The manifest's `schema_version` is not
    /// [`crate::share::BUNDLE_SCHEMA_VERSION`].
    SchemaVersion { found: u32, expected: u32 },
}

impl std::fmt::Display for RejectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RejectReason::Signature(detail) => write!(f, "signature: {detail}"),
            RejectReason::Checksum { expected, actual } => {
                write!(f, "checksum: expected {expected}, got {actual}")
            }
            RejectReason::SchemaVersion { found, expected } => {
                write!(f, "schema-version: found {found}, expected {expected}")
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
    pub at: String,
}

/// The outcome of a successful import: how many records/lessons were
/// added to the graph, all forced inactive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub records_imported: usize,
    pub lessons_imported: usize,
}

impl ImportReport {
    pub fn total(&self) -> usize {
        self.records_imported + self.lessons_imported
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
    bundle: &SignedBundle,
    trust_list: &TrustList,
    at: &str,
) -> Result<ImportReport, RejectedBundle> {
    verify_signature(bundle, trust_list).map_err(|reason| RejectedBundle {
        reason,
        at: at.to_owned(),
    })?;

    let (decompressed, snapshot) = decode_payload_unchecked(&bundle.compressed_payload)
        .map_err(|source| RejectedBundle {
            reason: RejectReason::Checksum {
                expected: bundle.manifest.content_hash.clone(),
                actual: format!("<undecodable: {source}>"),
            },
            at: at.to_owned(),
        })?;

    verify_checksum(&bundle.manifest.content_hash, &decompressed).map_err(|reason| {
        RejectedBundle {
            reason,
            at: at.to_owned(),
        }
    })?;

    verify_schema_version(bundle.manifest.schema_version).map_err(|reason| RejectedBundle {
        reason,
        at: at.to_owned(),
    })?;

    Ok(ingest_inactive(graph, snapshot))
}

fn verify_signature(bundle: &SignedBundle, trust_list: &TrustList) -> Result<(), RejectReason> {
    if !trust_list.is_trusted(&bundle.signer_public_key_hex) {
        return Err(RejectReason::Signature(format!(
            "signer key {} is not in the local trust list",
            bundle.signer_public_key_hex
        )));
    }
    let verifying_key = parse_verifying_key(&bundle.signer_public_key_hex)
        .ok_or_else(|| RejectReason::Signature("unparseable public key".to_owned()))?;
    let signature = parse_signature(&bundle.signature_hex)
        .ok_or_else(|| RejectReason::Signature("unparseable signature".to_owned()))?;
    verifying_key
        .verify_strict(&bundle.compressed_payload, &signature)
        .map_err(|source| RejectReason::Signature(format!("verification failed: {source}")))
}

fn verify_checksum(expected_hash: &str, decompressed_payload: &[u8]) -> Result<(), RejectReason> {
    let actual = enforcer_core::hash_chain::link_digest(None, decompressed_payload);
    if actual != expected_hash {
        return Err(RejectReason::Checksum {
            expected: expected_hash.to_owned(),
            actual,
        });
    }
    Ok(())
}

fn verify_schema_version(found: u32) -> Result<(), RejectReason> {
    if found != BUNDLE_SCHEMA_VERSION {
        return Err(RejectReason::SchemaVersion {
            found,
            expected: BUNDLE_SCHEMA_VERSION,
        });
    }
    Ok(())
}

/// Ingest every record/lesson in `snapshot` into `graph`, forcing each
/// one inactive (`landed_at` cleared) regardless of what the exporting
/// repo recorded.
fn ingest_inactive(graph: &mut MemoryGraph, snapshot: BundleGraphSnapshot) -> ImportReport {
    let mut report = ImportReport::default();
    for mut record in snapshot.records {
        record.landed_at.clear();
        graph.ingest_record(record);
        report.records_imported += 1;
    }
    for mut lesson in snapshot.lessons {
        lesson.landed_at.clear();
        graph.ingest_lesson_row(lesson);
        report.lessons_imported += 1;
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::share::{export_bundle, BundleGraphSnapshot, ExportConsent, Scope};
    use ed25519_dalek::SigningKey;
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
            landed_at: vec!["commit abc".to_string()],
            supersedes: None,
            provenance: crate::record::Provenance {
                writer: "primary".to_string(),
                ..Default::default()
            },
        });
        graph.ingest_lesson_row(crate::lesson::LessonRow {
            id: "L1".to_string(),
            date: "2026-07-05".to_string(),
            observed: "x".to_string(),
            lesson: "y".to_string(),
            landed_at: "commit def".to_string(),
            ships_via: "arc-16".to_string(),
        });
        BundleGraphSnapshot::from_graph(&graph)
    }

    fn signed_personal_bundle(key: &SigningKey) -> SignedBundle {
        export_bundle(
            &sample_snapshot(),
            Scope::Personal,
            ExportConsent::NotGranted,
            None,
            None,
            "2026-07-05T00:00:00Z",
            key,
        )
        .expect("export succeeds")
    }

    #[test]
    fn roundtrip_import_from_a_trusted_signer_succeeds() {
        let key = SigningKey::generate(&mut OsRng);
        let bundle = signed_personal_bundle(&key);
        let mut trust_list = TrustList::new();
        trust_list.trust(bundle.signer_public_key_hex.clone());

        let mut graph = MemoryGraph::new();
        let report = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z")
            .expect("trusted, unmodified bundle imports cleanly");
        assert_eq!(report.records_imported, 1);
        assert_eq!(report.lessons_imported, 1);
        assert_eq!(graph.len(), 2);
    }

    #[test]
    fn untrusted_signer_is_rejected_with_signature_reason() {
        let key = SigningKey::generate(&mut OsRng);
        let bundle = signed_personal_bundle(&key);
        let empty_trust_list = TrustList::new();

        let mut graph = MemoryGraph::new();
        let outcome = import_bundle(&mut graph, &bundle, &empty_trust_list, "2026-07-05T01:00:00Z");
        match outcome {
            Err(RejectedBundle {
                reason: RejectReason::Signature(_),
                ..
            }) => {}
            other => panic!("expected Signature rejection, got {other:?}"),
        }
        assert!(graph.is_empty(), "rejected import must not touch the graph");
    }

    #[test]
    fn tampered_payload_is_rejected_with_checksum_reason() {
        let key = SigningKey::generate(&mut OsRng);
        let mut bundle = signed_personal_bundle(&key);
        let mut trust_list = TrustList::new();
        trust_list.trust(bundle.signer_public_key_hex.clone());

        // Tamper with the manifest's recorded content hash so it no
        // longer matches the real decompressed payload -- simulating a
        // bundle whose payload was swapped after signing (the signature
        // still verifies because we did not touch `compressed_payload`,
        // so this specifically exercises the checksum gate, not the
        // signature gate).
        bundle.manifest.content_hash = format!("sha256:{}", "0".repeat(64));

        let mut graph = MemoryGraph::new();
        let outcome = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z");
        match outcome {
            Err(RejectedBundle {
                reason: RejectReason::Checksum { .. },
                ..
            }) => {}
            other => panic!("expected Checksum rejection, got {other:?}"),
        }
        assert!(graph.is_empty());
    }

    #[test]
    fn tampered_signed_bytes_are_rejected_with_signature_reason() {
        let key = SigningKey::generate(&mut OsRng);
        let mut bundle = signed_personal_bundle(&key);
        let mut trust_list = TrustList::new();
        trust_list.trust(bundle.signer_public_key_hex.clone());

        // Flip a byte in the signed compressed payload itself -- the
        // signature was computed over the ORIGINAL bytes, so this must
        // fail signature verification (checksum is never reached).
        if let Some(first) = bundle.compressed_payload.first_mut() {
            *first ^= 0xFF;
        }

        let mut graph = MemoryGraph::new();
        let outcome = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z");
        match outcome {
            Err(RejectedBundle {
                reason: RejectReason::Signature(_),
                ..
            }) => {}
            other => panic!("expected Signature rejection, got {other:?}"),
        }
        assert!(graph.is_empty());
    }

    #[test]
    fn wrong_schema_version_is_rejected() {
        let key = SigningKey::generate(&mut OsRng);
        let mut bundle = signed_personal_bundle(&key);
        // Re-sign is not needed to exercise this path: schema-version is
        // checked in the manifest, which is not itself signature-covered
        // (only compressed_payload is) -- see module docs on signing
        // scope. This directly demonstrates why checksum+schema are
        // separate gates from signature.
        bundle.manifest.schema_version = 999;
        let mut trust_list = TrustList::new();
        trust_list.trust(bundle.signer_public_key_hex.clone());

        let mut graph = MemoryGraph::new();
        let outcome = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z");
        match outcome {
            Err(RejectedBundle {
                reason: RejectReason::SchemaVersion { found: 999, .. },
                ..
            }) => {}
            other => panic!("expected SchemaVersion rejection, got {other:?}"),
        }
    }

    #[test]
    fn imported_lesson_is_inactive_until_local_validation_activates_it() {
        let key = SigningKey::generate(&mut OsRng);
        let bundle = signed_personal_bundle(&key);
        let mut trust_list = TrustList::new();
        trust_list.trust(bundle.signer_public_key_hex.clone());

        let mut graph = MemoryGraph::new();
        import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z")
            .expect("import succeeds");

        // The exporter's own graph had `landed_at = ["commit abc"]` for
        // mem-primary-0001 and landed_at = "commit def" for lesson L1 --
        // both must land INACTIVE here despite that, because this repo
        // has not locally validated them.
        assert_eq!(
            crate::learning::lesson_status(&graph, "mem-primary-0001"),
            Some(crate::learning::LessonStatus::Inactive),
            "imported record must be inactive even though the exporter had landed it"
        );
        assert_eq!(
            crate::learning::lesson_status(&graph, "L1"),
            Some(crate::learning::LessonStatus::Inactive),
            "imported lesson row must be inactive even though the exporter had landed it"
        );

        // Still searchable despite being inactive (crate-wide rule:
        // recall never filters by activation).
        let hits = crate::recall::recall(&graph, "sample statement");
        assert!(
            !hits.is_empty(),
            "inactive imported record must remain recall-searchable"
        );

        // Local x05 validation: `crate::learning`'s activation model
        // (X06.6, not owned by this subpack) keys a single id's status
        // off the FIRST node recorded under that id
        // (`lesson_status`'s `.find()`) -- re-ingesting the SAME id a
        // second time does not retroactively change its status. The
        // documented mechanism for "this repo now vouches for what it
        // imported" is `supersedes`: a NEW locally-landed record whose
        // `supersedes` names the imported id. That is exactly what
        // activates it here.
        graph.ingest_record(crate::record::MemoryRecord {
            schema_version: 1,
            id: "mem-primary-0001-validated".to_string(),
            ts: "2026-07-05T02:00:00Z".to_string(),
            kind: crate::record::RecordKind::Lesson,
            domain: crate::record::RecordDomain::Harness,
            statement: "sample statement".to_string(),
            why: None,
            how_to_apply: None,
            applies_to: vec![],
            evidence: None,
            routes: vec![],
            landed_at: vec!["local-commit-xyz".to_string()],
            supersedes: Some("mem-primary-0001".to_string()),
            provenance: crate::record::Provenance {
                writer: "primary".to_string(),
                ..Default::default()
            },
        });
        assert_eq!(
            crate::learning::lesson_status(&graph, "mem-primary-0001-validated"),
            Some(crate::learning::LessonStatus::Active),
            "the local validation record itself is active"
        );
        assert!(
            crate::learning::active_lessons(&graph).contains(&"mem-primary-0001-validated"),
            "the validated record must be the one counted as active"
        );
        assert!(
            !crate::learning::active_lessons(&graph).contains(&"mem-primary-0001"),
            "the superseded imported id must never be counted as active, \
             even though it now has a superseder that landed"
        );
        assert_eq!(
            crate::learning::superseded_by(&graph, "mem-primary-0001"),
            Some("mem-primary-0001-validated"),
            "the audit trail must record what superseded the imported id"
        );
    }

    #[test]
    fn graph_bootstrap_artifact_import_reconstructs_graph_counts() {
        // D-11: the team graph bootstrap artifact is this same bundle
        // format carrying the compressed graph -- importing it must
        // reconstruct the same node count the exporter's snapshot had.
        let key = SigningKey::generate(&mut OsRng);
        let snapshot = sample_snapshot();
        let expected_count = snapshot.node_count();
        let bundle = export_bundle(
            &snapshot,
            Scope::Team,
            ExportConsent::Granted,
            Some("team-bootstrap".to_string()),
            Some("deadbeef".to_string()),
            "2026-07-05T00:00:00Z",
            &key,
        )
        .expect("team bootstrap export succeeds");

        let mut trust_list = TrustList::new();
        trust_list.trust(bundle.signer_public_key_hex.clone());
        let mut graph = MemoryGraph::new();
        let report = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z")
            .expect("team bootstrap import succeeds");

        assert_eq!(report.total(), expected_count);
        assert_eq!(graph.len(), expected_count);
    }
}
