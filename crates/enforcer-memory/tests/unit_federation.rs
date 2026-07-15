use ed25519_dalek::SigningKey;
use rand_core::OsRng;

use enforcer_memory::federation::{
    import_bundle, import_bundle_logged, RejectReason, RejectedBundle, RejectionLog, TrustList,
};
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::record::{MemoryRecordDto as MemoryRecord, Provenance, RecordDomain, RecordKind};
use enforcer_memory::share::{
    export_bundle, BundleGraphSnapshot, ExportConsent, ExportRequest, Scope, SignedBundle,
};

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
        landed_at: vec!["commit abc".to_string()],
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
        landed_at: "commit def".to_string(),
        ships_via: "arc-16".to_string(),
    });
    BundleGraphSnapshot::from_graph(&graph)
}

fn signed_personal_bundle(
    key: &SigningKey,
) -> Result<SignedBundle, enforcer_memory::share::ShareError> {
    export_bundle(
        &sample_snapshot(),
        ExportRequest {
            scope: Scope::Personal,
            consent: ExportConsent::Granted,
            creator: None,
            git_head: None,
            created_at: "2026-07-05T00:00:00Z".to_string(),
        },
        key,
    )
}

#[test]
fn roundtrip_import_from_a_trusted_signer_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let key = SigningKey::generate(&mut OsRng);
    let bundle = signed_personal_bundle(&key)?;
    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    let mut graph = MemoryGraph::new();
    let report = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z")
        .map_err(|rejected| format!("expected success, got {rejected:?}"))?;
    assert_eq!(report.records_imported, 1);
    assert_eq!(report.lessons_imported, 1);
    assert_eq!(graph.len(), 2);
    Ok(())
}

#[test]
fn untrusted_signer_is_rejected_with_signature_reason() -> Result<(), Box<dyn std::error::Error>> {
    let key = SigningKey::generate(&mut OsRng);
    let bundle = signed_personal_bundle(&key)?;
    let empty_trust_list = TrustList::new();

    let mut graph = MemoryGraph::new();
    let mut log = RejectionLog::new();
    let outcome = import_bundle_logged(
        &mut graph,
        &bundle,
        &empty_trust_list,
        "2026-07-05T01:00:00Z",
        &mut log,
    );
    match outcome {
        Err(RejectedBundle {
            reason: RejectReason::UntrustedSigner(_),
            ..
        }) => {}
        other => return Err(format!("expected UntrustedSigner rejection, got {other:?}").into()),
    }
    assert!(graph.is_empty(), "rejected import must not touch the graph");
    assert_eq!(log.entries().len(), 1, "rejection must be recorded");
    Ok(())
}

#[test]
fn tampered_payload_is_rejected_with_checksum_reason() -> Result<(), Box<dyn std::error::Error>> {
    let key = SigningKey::generate(&mut OsRng);
    let mut bundle = signed_personal_bundle(&key)?;
    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    bundle.manifest.content_hash = format!("sha256:{}", "0".repeat(64));

    let mut graph = MemoryGraph::new();
    let outcome = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z");
    match outcome {
        Err(RejectedBundle {
            reason: RejectReason::ChecksumTamper { .. },
            ..
        }) => {}
        other => return Err(format!("expected ChecksumTamper rejection, got {other:?}").into()),
    }
    assert!(graph.is_empty());
    Ok(())
}

#[test]
fn tampered_signed_bytes_are_rejected_with_signature_reason(
) -> Result<(), Box<dyn std::error::Error>> {
    let key = SigningKey::generate(&mut OsRng);
    let mut bundle = signed_personal_bundle(&key)?;
    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    if let Some(first) = bundle.compressed_payload.first_mut() {
        *first ^= 0xFF;
    }

    let mut graph = MemoryGraph::new();
    let outcome = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z");
    match outcome {
        Err(RejectedBundle {
            reason: RejectReason::UntrustedSigner(_),
            ..
        }) => {}
        other => return Err(format!("expected UntrustedSigner rejection, got {other:?}").into()),
    }
    assert!(graph.is_empty());
    Ok(())
}

#[test]
fn wrong_schema_version_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let key = SigningKey::generate(&mut OsRng);
    let mut bundle = signed_personal_bundle(&key)?;
    bundle.manifest.schema_version = 999;
    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    let mut graph = MemoryGraph::new();
    let outcome = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z");
    match outcome {
        Err(RejectedBundle {
            reason: RejectReason::SchemaVersionIncompatible { found: 999, .. },
            ..
        }) => {}
        other => {
            return Err(
                format!("expected SchemaVersionIncompatible rejection, got {other:?}").into(),
            )
        }
    }
    Ok(())
}

#[test]
fn imported_lesson_is_inactive_until_local_validation_activates_it(
) -> Result<(), Box<dyn std::error::Error>> {
    let key = SigningKey::generate(&mut OsRng);
    let bundle = signed_personal_bundle(&key)?;
    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    let mut graph = MemoryGraph::new();
    import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z")
        .map_err(|rejected| format!("expected import to succeed, got {rejected:?}"))?;

    assert_eq!(
        enforcer_memory::learning::lesson_status(&graph, "mem-primary-0001"),
        Some(enforcer_memory::learning::LessonStatus::Inactive),
        "imported record must be inactive even though the exporter had landed it"
    );
    assert_eq!(
        enforcer_memory::learning::lesson_status(&graph, "L1"),
        Some(enforcer_memory::learning::LessonStatus::Inactive),
        "imported lesson row must be inactive even though the exporter had landed it"
    );

    let hits = enforcer_memory::recall::recall(&graph, "sample statement");
    assert!(
        !hits.is_empty(),
        "inactive imported record must remain recall-searchable"
    );

    graph.ingest_record(MemoryRecord {
        schema_version: 1,
        id: "mem-primary-0001-validated".to_string(),
        ts: "2026-07-05T02:00:00Z".to_string(),
        kind: RecordKind::Lesson,
        domain: RecordDomain::Harness,
        statement: "sample statement".to_string(),
        why: None,
        how_to_apply: None,
        applies_to: vec![],
        evidence: None,
        routes: vec![],
        landed_at: vec!["local-commit-xyz".to_string()],
        supersedes: Some("mem-primary-0001".to_string()),
        provenance: Provenance {
            writer: "primary".to_string(),
            ..Default::default()
        },
    });
    assert_eq!(
        enforcer_memory::learning::lesson_status(&graph, "mem-primary-0001-validated"),
        Some(enforcer_memory::learning::LessonStatus::Active),
        "the local validation record itself is active"
    );
    assert!(
        enforcer_memory::learning::active_lessons(&graph).contains(&"mem-primary-0001-validated"),
        "the validated record must be the one counted as active"
    );
    assert!(
        !enforcer_memory::learning::active_lessons(&graph).contains(&"mem-primary-0001"),
        "the superseded imported id must never be counted as active"
    );
    assert_eq!(
        enforcer_memory::learning::superseded_by(&graph, "mem-primary-0001"),
        Some("mem-primary-0001-validated"),
        "the audit trail must record what superseded the imported id"
    );
    Ok(())
}

#[test]
fn graph_bootstrap_artifact_import_reconstructs_graph_counts(
) -> Result<(), Box<dyn std::error::Error>> {
    let key = SigningKey::generate(&mut OsRng);
    let snapshot = sample_snapshot();
    let expected_count = snapshot.node_count();
    let bundle = export_bundle(
        &snapshot,
        ExportRequest {
            scope: Scope::Team,
            consent: ExportConsent::Granted,
            creator: Some("team-bootstrap".to_string()),
            git_head: Some("deadbeef".to_string()),
            created_at: "2026-07-05T00:00:00Z".to_string(),
        },
        &key,
    )?;

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());
    let mut graph = MemoryGraph::new();
    let report = import_bundle(&mut graph, &bundle, &trust_list, "2026-07-05T01:00:00Z")
        .map_err(|rejected| format!("expected import to succeed, got {rejected:?}"))?;

    assert_eq!(report.total(), expected_count);
    assert_eq!(graph.len(), expected_count);
    Ok(())
}
