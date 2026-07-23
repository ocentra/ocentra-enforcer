use ed25519_dalek::SigningKey;
use enforcer_domain::memory_types::{ExportConsent, MemoryShareScope};
use enforcer_domain::memory_types::{RecordDomain, RecordKind};
use enforcer_memory::boundary::record::MemoryRecordDto as MemoryRecord;
use enforcer_memory::boundary::record::ProvenanceDto;
use enforcer_memory::boundary::share::{
    export_bundle, BundleExportOptions, BundleGraphSnapshotDto, BundleManifestDto, LessonRowDto,
    ShareError, SignedBundleDto, BUNDLE_SCHEMA_VERSION,
};
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::lesson::LessonRow;
use rand_core::OsRng;

fn sample_snapshot() -> BundleGraphSnapshotDto {
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
        provenance: ProvenanceDto {
            writer: "primary".into(),
            ..Default::default()
        },
    });
    graph.ingest_lesson_row(LessonRow {
        id: "L1".to_string().into(),
        date: "2026-07-05".to_string().into(),
        observed: "x".to_string().into(),
        lesson: "y".to_string().into(),
        landed_at: "commit abc".to_string().into(),
        ships_via: "arc-16".to_string().into(),
    });
    BundleGraphSnapshotDto::from_graph(&graph)
}

fn request(
    scope: MemoryShareScope,
    consent: ExportConsent,
    creator: Option<String>,
) -> BundleExportOptions {
    BundleExportOptions {
        scope,
        consent,
        creator: creator.map(Into::into),
        git_head: None,
        created_at: "2026-07-05T00:00:00Z".to_string().into(),
    }
}

#[test]
fn personal_export_still_requires_consent() {
    let key = SigningKey::generate(&mut OsRng);
    let snapshot = sample_snapshot();
    let outcome = export_bundle(
        &snapshot,
        request(
            MemoryShareScope::Personal,
            ExportConsent::NotGranted,
            Some("primary".to_string()),
        ),
        &key,
    );
    assert!(matches!(
        outcome,
        Err(ShareError::ConsentRequired {
            scope: MemoryShareScope::Personal
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
            MemoryShareScope::Personal,
            ExportConsent::Granted,
            Some("primary".to_string()),
        ),
        &key,
    )?;
    assert_eq!(bundle.manifest.scope, MemoryShareScope::Personal);
    assert_eq!(
        bundle
            .manifest
            .creator
            .as_ref()
            .map(|creator| creator.as_str()),
        Some("primary")
    );
    Ok(())
}

#[test]
fn team_export_without_consent_is_rejected() {
    let key = SigningKey::generate(&mut OsRng);
    let snapshot = sample_snapshot();
    let outcome = export_bundle(
        &snapshot,
        request(MemoryShareScope::Team, ExportConsent::NotGranted, None),
        &key,
    );
    assert!(matches!(
        outcome,
        Err(ShareError::ConsentRequired {
            scope: MemoryShareScope::Team
        })
    ));
}

#[test]
fn team_export_with_consent_succeeds() -> Result<(), ShareError> {
    let key = SigningKey::generate(&mut OsRng);
    let snapshot = sample_snapshot();
    let mut req = request(
        MemoryShareScope::Team,
        ExportConsent::Granted,
        Some("team-lead".to_string()),
    );
    req.git_head = Some("abc123".to_string().into());
    let bundle = export_bundle(&snapshot, req, &key)?;
    assert_eq!(
        bundle.manifest.git_head.as_ref().map(|head| head.as_str()),
        Some("abc123")
    );
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
            MemoryShareScope::Community,
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
        request(MemoryShareScope::Personal, ExportConsent::Granted, None),
        &key,
    )?;
    let decompressed = zstd::decode_all(bundle.compressed_payload.as_slice())
        .map_err(ShareError::Decompression)?;
    let decoded: BundleGraphSnapshotDto = serde_json::from_slice(&decompressed)?;
    assert_eq!(decoded, snapshot);
    Ok(())
}

#[test]
fn signed_bundle_hex_payload_round_trips_through_serde() -> Result<(), serde_json::Error> {
    let bundle = SignedBundleDto {
        manifest: BundleManifestDto {
            schema_version: BUNDLE_SCHEMA_VERSION.into(),
            git_head: Some("abc123".to_string().into()),
            content_hash: "sha256:deadbeef".to_string().into(),
            scope: MemoryShareScope::Team,
            creator: Some("primary".to_string().into()),
            created_at: "2026-07-05T00:00:00Z".to_string().into(),
        },
        compressed_payload: vec![0, 1, 2, 255, 128, 17].into(),
        signature_hex: "00ff".to_string().into(),
        signer_public_key_hex: "11aa".to_string().into(),
    };

    let json = serde_json::to_string(&bundle)?;
    let decoded: SignedBundleDto = serde_json::from_str(&json)?;
    assert_eq!(decoded.compressed_payload, bundle.compressed_payload);
    assert_eq!(decoded.signature_hex, bundle.signature_hex);
    assert_eq!(decoded.signer_public_key_hex, bundle.signer_public_key_hex);
    Ok(())
}

#[test]
fn share_dtos_round_trip_as_one_canonical_bundle_contract() -> Result<(), Box<dyn std::error::Error>>
{
    let snapshot: BundleGraphSnapshotDto = sample_snapshot();
    let lesson: LessonRowDto = snapshot
        .lessons
        .first()
        .cloned()
        .ok_or("sample snapshot must contain a lesson")?;
    let manifest = BundleManifestDto {
        schema_version: BUNDLE_SCHEMA_VERSION.into(),
        git_head: Some("abc123".to_string().into()),
        content_hash: "sha256:deadbeef".to_string().into(),
        scope: MemoryShareScope::Team,
        creator: Some("primary".to_string().into()),
        created_at: "2026-07-05T00:00:00Z".to_string().into(),
    };
    let bundle = SignedBundleDto {
        manifest: manifest.clone(),
        compressed_payload: vec![0, 1, 2, 255].into(),
        signature_hex: "00ff".to_string().into(),
        signer_public_key_hex: "11aa".to_string().into(),
    };

    let lesson_back: LessonRowDto = serde_json::from_slice(&serde_json::to_vec(&lesson)?)?;
    let manifest_back: BundleManifestDto = serde_json::from_slice(&serde_json::to_vec(&manifest)?)?;
    let snapshot_back: BundleGraphSnapshotDto =
        serde_json::from_slice(&serde_json::to_vec(&snapshot)?)?;
    let bundle_back: SignedBundleDto = serde_json::from_slice(&serde_json::to_vec(&bundle)?)?;
    assert_eq!(lesson_back, lesson);
    assert_eq!(manifest_back, manifest);
    assert_eq!(snapshot_back, snapshot);
    assert_eq!(bundle_back, bundle);
    Ok(())
}
