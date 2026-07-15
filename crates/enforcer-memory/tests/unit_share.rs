use ed25519_dalek::SigningKey;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::record::{MemoryRecordDto as MemoryRecord, Provenance, RecordDomain, RecordKind};
use enforcer_memory::share::{
    export_bundle, BundleGraphSnapshot, BundleManifest, ExportConsent, ExportRequest, Scope,
    ShareError, SignedBundle, BUNDLE_SCHEMA_VERSION,
};
use rand_core::OsRng;

fn sample_snapshot() -> BundleGraphSnapshot {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(MemoryRecord {
        schema_version: 1,
        id: "mem-primary-0001".to_string(),
        ts: "2026-07-05T00:00:00Z".to_string(),
        kind: RecordKind::Lesson,
        domain: RecordDomain::Harness,
        statement: "sample statement".to_string(),
        why: None,
        how_to_apply: None,
        applies_to: vec![],
        evidence: None,
        routes: vec![],
        landed_at: vec![],
        supersedes: None,
        provenance: Provenance {
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
    assert_eq!(bundle.manifest.schema_version, BUNDLE_SCHEMA_VERSION);
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
    let decompressed = zstd::decode_all(bundle.compressed_payload.as_slice())
        .map_err(ShareError::Decompression)?;
    let decoded: BundleGraphSnapshot = serde_json::from_slice(&decompressed)?;
    assert_eq!(decoded, snapshot);
    Ok(())
}

#[test]
fn signed_bundle_hex_payload_round_trips_through_serde() -> Result<(), serde_json::Error> {
    let bundle = SignedBundle {
        manifest: BundleManifest {
            schema_version: BUNDLE_SCHEMA_VERSION,
            git_head: Some("abc123".to_string()),
            content_hash: "sha256:deadbeef".to_string(),
            scope: Scope::Team,
            creator: Some("primary".to_string()),
            created_at: "2026-07-05T00:00:00Z".to_string(),
        },
        compressed_payload: vec![0, 1, 2, 255, 128, 17],
        signature_hex: "00ff".to_string(),
        signer_public_key_hex: "11aa".to_string(),
    };

    let json = serde_json::to_string(&bundle)?;
    let decoded: SignedBundle = serde_json::from_str(&json)?;
    assert_eq!(decoded.compressed_payload, bundle.compressed_payload);
    assert_eq!(decoded.signature_hex, bundle.signature_hex);
    assert_eq!(decoded.signer_public_key_hex, bundle.signer_public_key_hex);
    Ok(())
}
