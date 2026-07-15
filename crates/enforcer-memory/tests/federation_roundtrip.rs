//! X06.8 integration test: sharing/federation/artifacts, exercised across
//! module boundaries the way a real caller would use them together --
//! `store::manifest::ArtifactManifest` -> `artifacts::get_exact`,
//! `graph::MemoryGraph` -> `share::export_bundle` ->
//! `federation::import_bundle`, `redaction::redact_record` against a
//! committed golden fixture, and the `.codebase-memory/graph.db.zst`
//! persistence artifact round trip. Per-module unit tests in
//! `src/artifacts.rs`/`src/share.rs`/`src/federation.rs`/`src/redaction.rs`
//! cover each module in isolation; this file is the hard-test list from
//! the workpack's spec, run end to end.

use ed25519_dalek::SigningKey;
use rand_core::OsRng;

use enforcer_memory::artifacts::{get_exact, ArtifactLookupError};
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use enforcer_memory::federation::{import_bundle, RejectReason, RejectedBundle, TrustList};
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ids::ArtifactId;
use enforcer_memory::learning::{lesson_status, LessonStatus};
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::boundary::record::MemoryRecordDto as MemoryRecord;
use enforcer_memory::record::{Provenance, RecordDomain, RecordKind};
use enforcer_memory::redaction::{redact_record, RedactionConfig};
use enforcer_memory::share::{export_bundle, BundleGraphSnapshot, ExportConsent, Scope};
use enforcer_memory::store::manifest::ArtifactManifest;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

fn run_git(dir: &std::path::Path, args: &[&str]) -> TestResult {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

fn init_git_repo(dir: &std::path::Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

fn commit_all(dir: &std::path::Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

// ---------------------------------------------------------------------
// 1+2+3: exact content-addressed retrieval, wrong-id fail-closed,
// traversal rejection.
// ---------------------------------------------------------------------

#[test]
fn exact_artifact_retrieval_wrong_id_and_traversal_are_all_fail_closed() -> TestResult {
    let root = temp_dir("artifacts");
    let mut manifest = ArtifactManifest::open(&root)?;
    let id = manifest.put(b"exact content", Some("a.txt"), "2026-07-05T00:00:00Z")?;

    // 1. Exact hit.
    let content = get_exact(&manifest, &id)?;
    assert_eq!(content, b"exact content");

    // 2. Wrong (well-formed but unknown) id must fail closed, never
    // return the artifact above as a "close enough" match.
    let wrong_id = ArtifactId::from_digest(format!("sha256:{}", "11".repeat(32)).parse()?);
    assert!(matches!(
        get_exact(&manifest, &wrong_id),
        Err(ArtifactLookupError::NotFound { .. })
    ));

    // 3. Traversal-shaped ids are rejected at the untrusted-text boundary,
    // before they can become an ArtifactId or reach the manifest.
    for traversal in ["../../secrets", "..\\..\\secrets", "/etc/passwd"] {
        assert!(traversal
            .parse::<enforcer_domain::hashes::Sha256>()
            .is_err());
    }

    std::fs::remove_dir_all(&root)?;
    Ok(())
}

// ---------------------------------------------------------------------
// 4: personal-scope export -> import roundtrip.
// ---------------------------------------------------------------------

#[test]
fn personal_bundle_export_import_roundtrips_exactly() -> TestResult {
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
        enforcer_memory::share::ExportRequest {
            scope: Scope::Personal,
            consent: ExportConsent::Granted,
            creator: Some("primary".to_string()),
            git_head: Some("deadbeef".to_string()),
            created_at: "2026-07-05T00:00:00Z".to_string(),
        },
        &key,
    )?;

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    let mut imported_graph = MemoryGraph::new();
    let report = import_bundle(
        &mut imported_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    )
    .map_err(|rejected| format!("expected success, got {rejected:?}"))?;

    assert_eq!(report.total(), 2);
    assert_eq!(imported_graph.len(), 2);
    Ok(())
}

// ---------------------------------------------------------------------
// 5: signature-mismatch rejection with typed reason.
// ---------------------------------------------------------------------

#[test]
fn tampering_the_signature_bytes_is_rejected_with_a_recorded_reason() -> TestResult {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(sample_record("mem-primary-1002", vec!["commit ccc"]));
    let snapshot = BundleGraphSnapshot::from_graph(&graph);

    let key = SigningKey::generate(&mut OsRng);
    let mut bundle = export_bundle(
        &snapshot,
        enforcer_memory::share::ExportRequest {
            scope: Scope::Personal,
            consent: ExportConsent::Granted,
            creator: None,
            git_head: None,
            created_at: "2026-07-05T00:00:00Z".to_string(),
        },
        &key,
    )?;

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    // Tamper with the signature hex itself (not the payload) -- this
    // isolates the signature check specifically.
    bundle.signature_hex = "00".repeat(64);

    let mut imported_graph = MemoryGraph::new();
    let outcome = import_bundle(
        &mut imported_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    );

    let rejected: RejectedBundle = match outcome {
        Err(rejected) => rejected,
        Ok(_) => return Err("tampered signature must be rejected".into()),
    };
    assert!(
        matches!(rejected.reason, RejectReason::UntrustedSigner(_)),
        "rejection reason must be recorded as UntrustedSigner, got {:?}",
        rejected.reason
    );
    assert_eq!(rejected.at, "2026-07-05T01:00:00Z");
    assert!(
        imported_graph.is_empty(),
        "a rejected bundle must never partially populate the graph"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// 6: checksum-tamper rejection (payload bytes modified independent of
// signature).
// ---------------------------------------------------------------------

#[test]
fn tampering_with_the_manifests_content_hash_is_rejected_as_a_checksum_failure() -> TestResult {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(sample_record("mem-primary-1003", vec!["commit ddd"]));
    let snapshot = BundleGraphSnapshot::from_graph(&graph);

    let key = SigningKey::generate(&mut OsRng);
    let mut bundle = export_bundle(
        &snapshot,
        enforcer_memory::share::ExportRequest {
            scope: Scope::Personal,
            consent: ExportConsent::Granted,
            creator: None,
            git_head: None,
            created_at: "2026-07-05T00:00:00Z".to_string(),
        },
        &key,
    )?;

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    // Corrupt the manifest's recorded content hash WITHOUT touching the
    // signed compressed bytes -- must fail at the checksum gate, proving
    // checksum verification is independent of signature verification.
    bundle.manifest.content_hash = format!("sha256:{}", "00".repeat(32));

    let mut imported_graph = MemoryGraph::new();
    let outcome = import_bundle(
        &mut imported_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    );
    let rejected: RejectedBundle = match outcome {
        Err(rejected) => rejected,
        Ok(_) => return Err("checksum-tampered manifest must be rejected".into()),
    };
    assert!(
        matches!(rejected.reason, RejectReason::ChecksumTamper { .. }),
        "expected ChecksumTamper rejection, got {:?}",
        rejected.reason
    );
    assert!(imported_graph.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------
// 7: imported lesson inactive until x05 landed-evidence path activates
// it.
// ---------------------------------------------------------------------

#[test]
fn imported_content_stays_inactive_until_a_local_landing_activates_it() -> TestResult {
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
        enforcer_memory::share::ExportRequest {
            scope: Scope::Team,
            consent: ExportConsent::Granted,
            creator: Some("exporter-team".to_string()),
            git_head: None,
            created_at: "2026-07-05T00:00:00Z".to_string(),
        },
        &key,
    )?;

    let mut trust_list = TrustList::new();
    trust_list.trust(bundle.signer_public_key_hex.clone());

    let mut local_graph = MemoryGraph::new();
    import_bundle(
        &mut local_graph,
        &bundle,
        &trust_list,
        "2026-07-05T01:00:00Z",
    )
    .map_err(|rejected| format!("expected import to succeed, got {rejected:?}"))?;

    // Despite the exporter's own landed_at, THIS repo has not validated
    // it locally yet -- must be inactive immediately after import.
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

    // The normal x05 landed-evidence path: land a NEW record whose
    // `supersedes` names the imported id -- this is how the crate's
    // existing (x06.6) activation model lets a repo vouch for imported
    // content, rather than re-ingesting the same id.
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
    Ok(())
}

// ---------------------------------------------------------------------
// 8: redaction golden byte-exact.
// ---------------------------------------------------------------------

#[test]
fn community_redaction_matches_the_committed_golden_fixture_byte_exact() -> TestResult {
    let fixture_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/memory/redaction");
    let input = std::fs::read_to_string(fixture_dir.join("community-input.ndjson"))?;
    let expected = std::fs::read_to_string(fixture_dir.join("community-expected.ndjson"))?;

    let record: MemoryRecord = serde_json::from_str(input.trim_end())?;
    let record = enforcer_memory::record::MemoryRecord::from_dto(record);
    let redacted = redact_record(
        &record,
        Some(r"C:\Projects\enforcer"),
        RedactionConfig::default(),
    );
    let actual = serde_json::to_string(&redacted.to_dto())? + "\n";
    assert_eq!(
        actual, expected,
        "community redaction output must be byte-exact against the committed golden fixture"
    );
    Ok(())
}

// ---------------------------------------------------------------------
// 9: code-graph artifact export -> bootstrap-import reconstructs
// identical node/edge counts on a fresh CodeGraph.
// ---------------------------------------------------------------------

#[test]
fn code_graph_artifact_export_then_bootstrap_import_reconstructs_identical_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    std::fs::write(&file_path, "fn a() {}\nfn b() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository_with_options(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
        enforcer_memory::code_graph::IndexOptions {
            mode: enforcer_memory::code_graph::IndexMode::Full,
            persistence: true,
            project_name: Some("demo"),
            indexed_at: Some("2026-07-05T00:00:00Z"),
        },
    )?;
    let expected_nodes = graph.nodes().len();
    let expected_edges = graph.imports().len() + graph.calls().len() + graph.routes().len();

    // A fresh CodeGraph, with an empty manifest, bootstraps purely from
    // the artifact already on disk.
    let mut bootstrapped = CodeGraph::new();
    bootstrapped.index_repository_with_options(
        dir.path(),
        &[],
        &Manifest::default(),
        enforcer_memory::code_graph::IndexOptions::default(),
    )?;

    assert_eq!(bootstrapped.nodes().len(), expected_nodes);
    let bootstrapped_edges =
        bootstrapped.imports().len() + bootstrapped.calls().len() + bootstrapped.routes().len();
    assert_eq!(bootstrapped_edges, expected_edges);
    Ok(())
}

// ---------------------------------------------------------------------
// 10: artifact.json field-parity assertion.
// ---------------------------------------------------------------------

#[test]
fn artifact_json_has_exactly_the_baseline_field_set_and_schema_version_two() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_git_repo(dir.path())?;
    let file_path = dir.path().join("a.rs");
    std::fs::write(&file_path, "fn a() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository_with_options(
        dir.path(),
        std::slice::from_ref(&file_path),
        &Manifest::default(),
        enforcer_memory::code_graph::IndexOptions {
            mode: enforcer_memory::code_graph::IndexMode::Full,
            persistence: true,
            project_name: Some("demo"),
            indexed_at: Some("2026-07-05T00:00:00Z"),
        },
    )?;

    let meta_path = dir.path().join(".codebase-memory").join("artifact.json");
    let raw = std::fs::read_to_string(&meta_path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let obj = value
        .as_object()
        .ok_or("artifact.json must be a JSON object")?;

    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    let mut expected = vec![
        "schema_version",
        "commit",
        "indexed_at",
        "project",
        "nodes",
        "edges",
        "original_size",
        "compressed_size",
        "compression_level",
    ];
    expected.sort_unstable();
    assert_eq!(
        keys, expected,
        "artifact.json field set must be exactly the baseline's set, no more no fewer"
    );
    assert_eq!(obj["schema_version"], serde_json::json!(2));
    Ok(())
}
