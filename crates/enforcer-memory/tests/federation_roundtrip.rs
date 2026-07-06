//! X06.8 integration test: sharing/federation/artifacts, exercised across
//! crate boundaries the way a real caller would use them together --
//! `store::manifest::ArtifactManifest` -> `artifacts::get_exact`,
//! `graph::MemoryGraph` -> `share::export_bundle` ->
//! `federation::import_bundle`, and `redaction::redact_record` against a
//! committed golden fixture. The per-module unit tests in
//! `src/artifacts.rs`/`src/share.rs`/`src/federation.rs`/`src/redaction.rs`
//! cover each module in isolation; this file is the hard-test list from
//! the workpack's X06.8 pack, run end to end.

use ed25519_dalek::SigningKey;
use rand_core::OsRng;

use enforcer_memory::artifacts::{get_exact, ArtifactLookupError};
use enforcer_memory::federation::{import_bundle, RejectReason, RejectedBundle, TrustList};
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::learning::{lesson_status, LessonStatus};
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::record::{MemoryRecord, Provenance, RecordDomain, RecordKind};
use enforcer_memory::redaction::{redact_record, RedactionConfig};
use enforcer_memory::share::{export_bundle, BundleGraphSnapshot, ExportConsent, Scope};
use enforcer_memory::store::manifest::ArtifactManifest;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let unique = format!(
        "enforcer-memory-federation-roundtrip-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

fn sample_record(id: &str, landed_at: Vec<&str>) -> MemoryRecord {
    MemoryRecord {
        schema_version: 1,
        id: id.to_string(),
        ts: "2026-07-05T00:00:00Z".to_string(),
        kind: RecordKind::Lesson,
        domain: RecordDomain::Harness,
        statement: format!("statement for {id}"),
        why: None,
        how_to_apply: None,
        applies_to: vec![],
        evidence: None,
        routes: vec![],
        landed_at: landed_at.into_iter().map(String::from).collect(),
        supersedes: None,
        provenance: Provenance {
            writer: "primary".to_string(),
            ..Default::default()
        },
    }
}

// ---------------------------------------------------------------------
// Exact artifact retrieval + wrong-id fail-closed + traversal rejection
// ---------------------------------------------------------------------

#[test]
fn exact_artifact_retrieval_wrong_id_and_traversal_are_all_fail_closed() {
    let root = temp_dir("artifacts");
    let mut manifest = ArtifactManifest::open(&root).expect("open manifest");
    let id = manifest
        .put(b"exact content", Some("a.txt"), "2026-07-05T00:00:00Z")
        .expect("put");

    // Exact hit.
    let content = get_exact(&manifest, id.as_str()).expect("exact id must resolve");
    assert_eq!(content, b"exact content");

    // Wrong (well-formed but unknown) id must fail closed, never return
    // the artifact above as a "close enough" match.
    let wrong_id = format!("sha256:{}", "11".repeat(32));
    assert!(matches!(
        get_exact(&manifest, &wrong_id),
        Err(ArtifactLookupError::NotFound { .. })
    ));

    // Traversal-shaped ids are rejected outright.
    for traversal in ["../../secrets", "..\\..\\secrets", "/etc/passwd"] {
        assert!(matches!(
            get_exact(&manifest, traversal),
            Err(ArtifactLookupError::TraversalRejected { .. })
        ));
    }

    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------
// Bundle export -> import roundtrip (personal scope, default)
// ---------------------------------------------------------------------

#[test]
fn personal_bundle_export_import_roundtrips_without_consent() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(sample_record("mem-primary-1001", vec!["commit aaa"]));
    graph.ingest_lesson_row(LessonRow {
        id: "L100".to_string(),
        date: "2026-07-05".to_string(),
        observed: "observed x".to_string(),
        lesson: "learned y".to_string(),
        landed_at: "commit bbb".to_string(),
        ships_via: "arc-16".to_string(),
    });
    let snapshot = BundleGraphSnapshot::from_graph(&graph);
    assert_eq!(snapshot.node_count(), 2);

    let key = SigningKey::generate(&mut OsRng);
    let bundle = export_bundle(
        &snapshot,
        Scope::Personal,
        ExportConsent::NotGranted,
        Some("primary".to_string()),
        Some("deadbeef".to_string()),
        "2026-07-05T00:00:00Z",
        &key,
    )
    .expect("personal export needs no consent");

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    let mut imported_graph = MemoryGraph::new();
    let report = import_bundle(
        &mut imported_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    )
    .expect("trusted personal bundle imports cleanly");

    assert_eq!(report.total(), 2);
    assert_eq!(imported_graph.len(), 2);
}

// ---------------------------------------------------------------------
// Signature-mismatch rejection with reason
// ---------------------------------------------------------------------

#[test]
fn bundle_from_an_untrusted_signer_is_rejected_with_a_recorded_reason() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(sample_record("mem-primary-1002", vec!["commit ccc"]));
    let snapshot = BundleGraphSnapshot::from_graph(&graph);

    let untrusted_key = SigningKey::generate(&mut OsRng);
    let bundle = export_bundle(
        &snapshot,
        Scope::Personal,
        ExportConsent::NotGranted,
        None,
        None,
        "2026-07-05T00:00:00Z",
        &untrusted_key,
    )
    .expect("export succeeds even though this signer will not be trusted");

    // An empty trust list: nobody is trusted.
    let trust_list = TrustList::new();
    let mut imported_graph = MemoryGraph::new();
    let outcome = import_bundle(
        &mut imported_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    );

    let rejected = outcome.expect_err("untrusted signer must be rejected, not imported");
    assert!(
        matches!(rejected.reason, RejectReason::Signature(_)),
        "rejection reason must be recorded as Signature, got {:?}",
        rejected.reason
    );
    assert_eq!(rejected.at, "2026-07-05T01:00:00Z");
    assert!(
        imported_graph.is_empty(),
        "a rejected bundle must never partially populate the graph"
    );
}

// ---------------------------------------------------------------------
// Checksum-tamper rejection
// ---------------------------------------------------------------------

#[test]
fn tampering_with_the_manifests_content_hash_is_rejected_as_a_checksum_failure() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(sample_record("mem-primary-1003", vec!["commit ddd"]));
    let snapshot = BundleGraphSnapshot::from_graph(&graph);

    let key = SigningKey::generate(&mut OsRng);
    let mut bundle = export_bundle(
        &snapshot,
        Scope::Personal,
        ExportConsent::NotGranted,
        None,
        None,
        "2026-07-05T00:00:00Z",
        &key,
    )
    .expect("export succeeds");

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    // Corrupt the manifest's recorded content hash WITHOUT touching the
    // signed compressed bytes -- this must fail at the checksum gate,
    // proving checksum verification is independent of signature
    // verification (both must hold, neither substitutes for the other).
    bundle.manifest.content_hash = format!("sha256:{}", "00".repeat(32));

    let mut imported_graph = MemoryGraph::new();
    let outcome = import_bundle(
        &mut imported_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    );
    let rejected: RejectedBundle =
        outcome.expect_err("checksum-tampered manifest must be rejected");
    assert!(
        matches!(rejected.reason, RejectReason::Checksum { .. }),
        "expected Checksum rejection, got {:?}",
        rejected.reason
    );
    assert!(imported_graph.is_empty());
}

// ---------------------------------------------------------------------
// Imported lesson inactive until x05 validation; activation flips it
// ---------------------------------------------------------------------

#[test]
fn imported_content_stays_inactive_until_a_local_landing_activates_it() {
    let mut exporter_graph = MemoryGraph::new();
    // The EXPORTER's own repo already landed this -- landed_at is
    // non-empty on their side.
    exporter_graph.ingest_record(sample_record(
        "mem-primary-1004",
        vec!["exporter-commit-123"],
    ));
    let snapshot = BundleGraphSnapshot::from_graph(&exporter_graph);

    let key = SigningKey::generate(&mut OsRng);
    let bundle = export_bundle(
        &snapshot,
        Scope::Team,
        ExportConsent::Granted,
        Some("exporter-team".to_string()),
        None,
        "2026-07-05T00:00:00Z",
        &key,
    )
    .expect("consented team export succeeds");

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    let mut local_graph = MemoryGraph::new();
    import_bundle(
        &mut local_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    )
    .expect("trusted team bundle imports");

    // Despite the exporter's own landed_at, THIS repo has not validated
    // it locally yet -- must be inactive.
    assert_eq!(
        lesson_status(&local_graph, "mem-primary-1004"),
        Some(LessonStatus::Inactive)
    );

    // Still searchable while inactive (searchable-but-inactive rule).
    let hits = enforcer_memory::recall::recall(&local_graph, "statement for mem-primary-1004");
    assert!(
        !hits.is_empty(),
        "inactive imported record must remain recall-searchable"
    );

    // Local x05 validation: `crate::learning::lesson_status` keys a
    // single id's status off the FIRST node recorded under that id, so
    // re-ingesting "mem-primary-1004" again would not itself flip
    // anything. The existing (X06.6) supersede mechanism is how this
    // repo vouches for what it imported: land a NEW record whose
    // `supersedes` names the imported id.
    local_graph.ingest_record(MemoryRecord {
        supersedes: Some("mem-primary-1004".to_string()),
        landed_at: vec!["local-validation-commit-456".to_string()],
        ..sample_record("mem-primary-1004-validated", vec![])
    });
    assert_eq!(
        lesson_status(&local_graph, "mem-primary-1004-validated"),
        Some(LessonStatus::Active),
        "the local validation record is active"
    );
    let active = enforcer_memory::learning::active_lessons(&local_graph);
    assert!(
        active.contains(&"mem-primary-1004-validated"),
        "the validated record must be counted as active"
    );
    assert!(
        !active.contains(&"mem-primary-1004"),
        "the superseded imported id must never be counted as active"
    );
}

// ---------------------------------------------------------------------
// Redaction golden: byte-exact
// ---------------------------------------------------------------------

#[test]
fn community_redaction_matches_the_committed_golden_fixture_byte_exact() {
    let fixture_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/memory/redaction");
    let input = std::fs::read_to_string(fixture_dir.join("community-input.ndjson"))
        .expect("read golden input fixture");
    let expected = std::fs::read_to_string(fixture_dir.join("community-expected.ndjson"))
        .expect("read golden expected fixture");

    let record: MemoryRecord = serde_json::from_str(input.trim_end()).expect("parse fixture");
    let redacted = redact_record(
        &record,
        Some(r"C:\Projects\enforcer"),
        RedactionConfig::default(),
    );
    let actual = serde_json::to_string(&redacted).expect("serialize") + "\n";
    assert_eq!(
        actual, expected,
        "community redaction output must be byte-exact against the committed golden fixture"
    );
}

// ---------------------------------------------------------------------
// D-11: graph bootstrap artifact import reconstructs graph counts
// ---------------------------------------------------------------------

#[test]
fn team_graph_bootstrap_artifact_import_reconstructs_the_exporters_node_count() {
    let mut exporter_graph = MemoryGraph::new();
    for n in 0..5 {
        exporter_graph.ingest_record(sample_record(
            &format!("mem-primary-2{n:03}"),
            vec![format!("commit-{n}").as_str()],
        ));
    }
    for n in 0..3 {
        exporter_graph.ingest_lesson_row(LessonRow {
            id: format!("L2{n}"),
            date: "2026-07-05".to_string(),
            observed: "observed".to_string(),
            lesson: "learned".to_string(),
            landed_at: format!("commit-lesson-{n}"),
            ships_via: "arc-16".to_string(),
        });
    }
    let snapshot = BundleGraphSnapshot::from_graph(&exporter_graph);
    let expected_count = snapshot.node_count();
    assert_eq!(expected_count, 8, "5 records + 3 lessons");
    assert_eq!(
        expected_count,
        exporter_graph.len(),
        "snapshot node_count must match the exporter's own graph length \
         (Incident nodes excluded on both sides -- none exist here)"
    );

    let key = SigningKey::generate(&mut OsRng);
    let bundle = export_bundle(
        &snapshot,
        Scope::Team,
        ExportConsent::Granted,
        Some("team-bootstrap".to_string()),
        Some("bootstrap-head-sha".to_string()),
        "2026-07-05T00:00:00Z",
        &key,
    )
    .expect("team bootstrap export succeeds");

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    // A brand-new teammate's empty graph, bootstrapped purely from the
    // artifact.
    let mut new_teammate_graph = MemoryGraph::new();
    let report = import_bundle(
        &mut new_teammate_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    )
    .expect("team bootstrap artifact imports cleanly");

    assert_eq!(report.total(), expected_count);
    assert_eq!(new_teammate_graph.len(), expected_count);
}
