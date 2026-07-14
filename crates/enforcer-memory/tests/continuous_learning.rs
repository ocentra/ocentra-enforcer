//! X06.6 hard tests: the continuous-learning slice's acceptance surface,
//! exercised end to end through the public API against a fixture NDJSON
//! corpus (`tests/fixtures/memory/continuous-learning.ndjson`) rather
//! than each module's own inline unit tests. Every hard test named in
//! the workpack's acceptance block is represented here by exactly one
//! `#[test]` function so the mapping from requirement to test is
//! unambiguous:
//!
//! 1. observation exists per operation type (scan/check/run/doctor/closeout)
//! 2. clean-scan negative evidence
//! 3. recurrence curve update after landing
//! 4. lesson activation rules (active/inactive/unknown)
//! 5. supersede handling
//! 6. route-choice trace with confidence
//! 7. improvement curve (per-domain learning curve) emission

use enforcer_domain::paths::RepoRoot;
use enforcer_memory::evidence::{
    evidence_chain, recurrence_curve, EvidenceReport, NoProofRefs, ProofRefLookup,
};
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{
    ingest_ndjson_into, ingest_observation, ingest_observation_into_store, Observation,
};
use enforcer_memory::learning::{
    active_lessons, learning_curve, lesson_status, project_learning_from_store, superseded_by,
    LessonStatus,
};
use enforcer_memory::observations::{
    procedural_success_rate, record_procedural, record_procedural_in_store, record_route_choice,
    record_route_choice_in_store, ProceduralOutcome, ProceduralStoreInput, RouteChoiceStoreInput,
};
use enforcer_memory::record::RecordDomain;
use enforcer_memory::sessionstart::recall_pack;
use enforcer_memory::store::Store;
use std::collections::HashMap;

fn load_fixture_graph() -> Result<MemoryGraph, Box<dyn std::error::Error>> {
    let ndjson = include_str!("fixtures/memory/continuous-learning.ndjson");
    let mut graph = MemoryGraph::new();
    let ingested = ingest_ndjson_into(&mut graph, ndjson)?;
    assert_eq!(ingested, 4, "fixture corpus has exactly 4 records");
    Ok(graph)
}

/// Hard test 1: every scan/check/run/doctor/closeout call site writes an
/// observation via the usage-ingestion seam -- one Incident per
/// operation type, all independently retrievable.
#[test]
fn observation_recorded_per_operation_type() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = load_fixture_graph()?;
    let surfaces = ["scan", "check", "run", "doctor", "closeout"];
    let mut ids = Vec::new();
    for surface in surfaces {
        let id = ingest_observation(
            &mut graph,
            Observation {
                lesson_id: "mem-cl-0001".to_string(),
                rule_id: Some("CL-UNKNOWN-RULE".to_string()),
                fault_class: Some("unknown_rule_id".to_string()),
                repo_context: "crates/enforcer-scan".to_string(),
                clean: false,
                source_surface: surface.to_string(),
                ts: "2026-07-05T10:00:00Z".to_string(),
            },
        );
        assert!(
            id.starts_with(&format!("obs-{surface}-")),
            "observation id must record its source surface, got {id}"
        );
        ids.push(id);
    }
    assert_eq!(ids.len(), 5, "one observation per operation type");
    let incidents = graph.incidents_for_lesson("mem-cl-0001");
    assert_eq!(
        incidents.len(),
        5,
        "all five operation-type observations must be retrievable for the lesson they concern"
    );
    for surface in surfaces {
        assert!(
            incidents.iter().any(|inc| inc.source_surface == surface),
            "missing an observation from surface {surface}"
        );
    }
    Ok(())
}

/// Hard test 2: a clean run (no finding) still writes an observation --
/// negative evidence that the fault class was absent, not silence.
#[test]
fn clean_scan_writes_negative_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = load_fixture_graph()?;
    let before = graph.len();
    let id = ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-cl-0003".to_string(),
            rule_id: None,
            fault_class: None,
            repo_context: "crates/enforcer-check".to_string(),
            clean: true,
            source_surface: "check".to_string(),
            ts: "2026-07-05T10:01:00Z".to_string(),
        },
    );
    assert_eq!(
        graph.len(),
        before + 1,
        "a clean run must still append an observation, not be skipped"
    );
    let incidents = graph.incidents_for_lesson("mem-cl-0003");
    match incidents.iter().find(|inc| inc.id == id) {
        Some(clean_incident) => assert!(
            clean_incident.clean,
            "the observation must be recorded as clean = true (negative evidence)"
        ),
        None => {
            return Err(
                "test setup: just-ingested incident must be retrievable by lesson id".into(),
            )
        }
    }

    // Negative evidence must be visible through the same evidence chain
    // as any other observation -- it is not a second-class record.
    match evidence_chain(&graph, "mem-cl-0003", &NoProofRefs) {
        EvidenceReport::Chain { observed, .. } => {
            assert!(
                observed.iter().any(|o| o.incident.id == id),
                "clean observation must appear in the t0 evidence chain"
            );
        }
        EvidenceReport::Unknown { .. } => return Err("mem-cl-0003 is a known lesson".into()),
    }
    Ok(())
}

/// Hard test 3: the recurrence curve updates after a lesson lands --
/// incidents recorded once landing evidence exists count toward a
/// running "since landing" total; incidents before any landing do not.
#[test]
fn recurrence_curve_updates_after_landing() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = load_fixture_graph()?;
    // mem-cl-0001 is already landed in the fixture (landedAt = ["commit
    // cl-0001-fix"]). Every incident recorded against it from here on
    // counts as recurrence since landing.
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-cl-0001".to_string(),
            rule_id: Some("CL-UNKNOWN-RULE".to_string()),
            fault_class: Some("unknown_rule_id".to_string()),
            repo_context: "crates/enforcer-scan".to_string(),
            clean: false,
            source_surface: "scan".to_string(),
            ts: "2026-07-05T10:02:00Z".to_string(),
        },
    );
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-cl-0001".to_string(),
            rule_id: Some("CL-UNKNOWN-RULE".to_string()),
            fault_class: Some("unknown_rule_id".to_string()),
            repo_context: "crates/enforcer-scan".to_string(),
            clean: false,
            source_surface: "check".to_string(),
            ts: "2026-07-05T10:03:00Z".to_string(),
        },
    );
    let curve = recurrence_curve(&graph, "mem-cl-0001");
    assert_eq!(curve.len(), 2, "two incidents recorded after landing");
    assert!(curve[0].since_landing);
    assert!(curve[1].since_landing);
    assert_eq!(curve[0].running_recurrence_count, 1);
    assert_eq!(
        curve[1].running_recurrence_count, 2,
        "recurrence count must run cumulatively, not reset per incident"
    );

    // mem-cl-0004 has no landing evidence at all -- an incident against
    // it never counts as recurrence.
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-cl-0004".to_string(),
            rule_id: None,
            fault_class: Some("x".to_string()),
            repo_context: "crates/enforcer-memory".to_string(),
            clean: false,
            source_surface: "scan".to_string(),
            ts: "2026-07-05T10:04:00Z".to_string(),
        },
    );
    let unlanded_curve = recurrence_curve(&graph, "mem-cl-0004");
    assert_eq!(unlanded_curve.len(), 1);
    assert!(
        !unlanded_curve[0].since_landing,
        "no landing exists yet for mem-cl-0004 -- must not be counted as recurrence"
    );
    Ok(())
}

/// Hard test 4: lesson activation rules -- landed/proof-linked lessons
/// are Active, unlanded/imported lessons are Inactive-but-searchable,
/// and an id with no matching node at all is Unknown (`None`), never
/// silently treated as Inactive.
#[test]
fn lesson_activation_rules() -> Result<(), Box<dyn std::error::Error>> {
    let graph = load_fixture_graph()?;

    assert_eq!(
        lesson_status(&graph, "mem-cl-0001"),
        Some(LessonStatus::Active),
        "landed lesson must be active"
    );
    assert_eq!(
        lesson_status(&graph, "mem-cl-0004"),
        Some(LessonStatus::Inactive),
        "unlanded/imported lesson must be inactive, not unknown"
    );
    assert_eq!(
        lesson_status(&graph, "mem-cl-does-not-exist"),
        None,
        "an id with no matching node must be Unknown, distinct from Inactive"
    );

    // Inactive lessons stay searchable: recall never filters by
    // activation status.
    let hits = enforcer_memory::recall::recall(&graph, "imported");
    let ids: Vec<&str> = hits.iter().map(|hit| hit.node.id()).collect();
    assert!(
        ids.contains(&"mem-cl-0004"),
        "an inactive lesson must remain recall-searchable, got {ids:?}"
    );
    Ok(())
}

/// Hard test 5: supersede handling -- a superseding record retires the
/// earlier record's activation even though the earlier record still has
/// its own landing evidence; `superseded_by` answers the audit-trail
/// question without deleting/mutating the superseded record.
#[test]
fn supersede_handling() -> Result<(), Box<dyn std::error::Error>> {
    let graph = load_fixture_graph()?;

    // mem-cl-0002 independently satisfies the landed-evidence rule
    // (landedAt = ["commit cl-0002-partial-fix"]) but is superseded by
    // mem-cl-0003.
    assert_eq!(
        lesson_status(&graph, "mem-cl-0002"),
        Some(LessonStatus::Active),
        "mem-cl-0002's own landing evidence still makes lesson_status Active in isolation"
    );

    let active = active_lessons(&graph);
    assert!(
        !active.contains(&"mem-cl-0002"),
        "active_lessons must exclude a superseded lesson even if independently landed"
    );
    assert!(
        active.contains(&"mem-cl-0003"),
        "the superseding lesson must itself be active"
    );
    assert_eq!(
        superseded_by(&graph, "mem-cl-0002"),
        Some("mem-cl-0003"),
        "audit trail: superseded_by must answer what replaced mem-cl-0002"
    );
    assert_eq!(
        superseded_by(&graph, "mem-cl-0003"),
        None,
        "mem-cl-0003 has not itself been superseded"
    );

    // Append-only: the superseded record is still present in the graph,
    // never deleted or mutated.
    assert!(
        graph.nodes().iter().any(|n| n.id() == "mem-cl-0002"),
        "superseded record must remain in the append-only graph"
    );
    Ok(())
}

/// Hard test 6: meta-memory records which retrieval route a query took
/// and how confident that route choice was; confidence is clamped into
/// [0.0, 1.0] rather than silently storing an out-of-range value.
#[test]
fn route_choice_trace_with_confidence() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = load_fixture_graph()?;

    let id = record_route_choice(
        &mut graph,
        "unknown rule id hard error",
        "recall",
        0.85,
        "2026-07-05T10:05:00Z",
    );
    assert!(id.starts_with("route-"));
    assert_eq!(graph.route_traces().len(), 1);
    assert_eq!(graph.route_traces()[0].route, "recall");
    assert_eq!(graph.route_traces()[0].confidence, 0.85);

    // Out-of-range confidence must be clamped, never stored raw.
    record_route_choice(
        &mut graph,
        "q-high",
        "evidence",
        3.0,
        "2026-07-05T10:06:00Z",
    );
    record_route_choice(
        &mut graph,
        "q-low",
        "evidence",
        -2.0,
        "2026-07-05T10:07:00Z",
    );
    assert_eq!(graph.route_traces()[1].confidence, 1.0);
    assert_eq!(graph.route_traces()[2].confidence, 0.0);

    record_route_choice(
        &mut graph,
        "malformed confidence",
        "recall",
        f64::NAN,
        "2026-01-01T00:00:03Z",
    );
    assert_eq!(graph.route_traces()[3].confidence, 0.0);

    // Procedural memory records both retrieval/fix success AND failure
    // for the same lesson -- a memory that only logs success cannot
    // distinguish "reliable" from "tried once and got lucky".
    record_procedural(
        &mut graph,
        "mem-cl-0003",
        ProceduralOutcome::FixSuccess,
        "applied hard-error-on-unknown-rule-id fix to enforcer-check",
        "2026-07-05T10:08:00Z",
    );
    record_procedural(
        &mut graph,
        "mem-cl-0003",
        ProceduralOutcome::FixFailure,
        "fix regressed under a different rule id shape",
        "2026-07-05T10:09:00Z",
    );
    assert_eq!(graph.procedural_records().len(), 2);
    assert_eq!(
        procedural_success_rate(&graph, "mem-cl-0003"),
        Some(0.5),
        "one success and one failure must average to a 0.5 success rate"
    );
    assert_eq!(
        procedural_success_rate(&graph, "mem-cl-never-tried"),
        None,
        "no procedural history at all must be None, distinct from Some(0.0)"
    );
    Ok(())
}

/// Hard test 7: the aggregate improvement/learning curve -- per-domain
/// running counts of landed lessons and cumulative incidents, computed
/// purely from the graph's own append-only history.
#[test]
fn improvement_curve_emission() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = load_fixture_graph()?;

    // Give mem-cl-0001 (domain harness) two incidents, and mem-cl-0003
    // (domain harness, the superseder) one incident, so the curve has
    // real cumulative-incident data to report.
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-cl-0001".to_string(),
            rule_id: Some("CL-UNKNOWN-RULE".to_string()),
            fault_class: Some("unknown_rule_id".to_string()),
            repo_context: "crates/enforcer-scan".to_string(),
            clean: false,
            source_surface: "scan".to_string(),
            ts: "2026-07-05T10:10:00Z".to_string(),
        },
    );
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-cl-0003".to_string(),
            rule_id: Some("CL-UNKNOWN-RULE".to_string()),
            fault_class: Some("unknown_rule_id".to_string()),
            repo_context: "crates/enforcer-check".to_string(),
            clean: false,
            source_surface: "check".to_string(),
            ts: "2026-07-05T10:11:00Z".to_string(),
        },
    );

    let curves: HashMap<RecordDomain, _> = learning_curve(&graph);
    match curves.get(&RecordDomain::Harness) {
        Some(harness) => {
            // mem-cl-0001 and mem-cl-0003 both land in the harness domain
            // (mem-cl-0002 also lands in harness but the curve does not
            // exclude superseded records -- learning_curve tracks landing
            // history, not current activation; supersede handling is
            // `active_lessons`' job).
            assert!(
                harness.iter().any(|p| p.lesson_id == "mem-cl-0001"),
                "harness curve must include mem-cl-0001"
            );
            let Some(last) = harness.last() else {
                return Err("harness curve is non-empty".into());
            };
            assert_eq!(
                last.landed_count,
                harness.len(),
                "landed_count must run cumulatively across the domain's own points"
            );
        }
        None => return Err("harness domain has landed lessons in the fixture".into()),
    }

    // mem-cl-0004 (domain code, unlanded) must not appear in any curve.
    let code = curves.get(&RecordDomain::Code);
    assert!(
        code.is_none(),
        "an unlanded lesson must not fabricate a code-domain curve entry"
    );
    Ok(())
}

#[test]
fn store_backed_learning_projection_replays_t0_t1_t2() -> Result<(), Box<dyn std::error::Error>> {
    let seed_graph = load_fixture_graph()?;
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-continuous-learning-store".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-07T00:00:00Z")?;
    let mut write_projection = MemoryGraph::new();

    ingest_observation_into_store(
        &mut store,
        &mut write_projection,
        Observation {
            lesson_id: "mem-cl-0001".to_string(),
            rule_id: Some("CL-STORE".to_string()),
            fault_class: Some("store-backed-recurrence".to_string()),
            repo_context: "crates/enforcer-memory/src/learning.rs".to_string(),
            clean: false,
            source_surface: "check".to_string(),
            ts: "2026-07-07T00:01:00Z".to_string(),
        },
    )?;
    record_procedural_in_store(
        &mut store,
        &mut write_projection,
        &ProceduralStoreInput::new(
            "mem-cl-0001",
            ProceduralOutcome::FixSuccess,
            "confirmed Store replay drives recurrence curve",
            "2026-07-07T00:02:00Z",
        ),
    )?;
    record_route_choice_in_store(
        &mut store,
        &mut write_projection,
        &RouteChoiceStoreInput::new(
            "continuous learning store projection",
            "learning-projection",
            0.88,
            "2026-07-07T00:03:00Z",
        ),
    )?;

    assert!(
        seed_graph.incidents_for_lesson("mem-cl-0001").is_empty(),
        "fixture seed graph carries lessons only; Store replay supplies t0 observations"
    );

    let projection = project_learning_from_store(&store, &seed_graph)?;
    assert_eq!(projection.replayed_incident_observations, 1);
    assert_eq!(projection.replayed_procedural_and_routes, 2);
    assert_eq!(projection.procedural_record_count, 1);
    assert_eq!(projection.route_trace_count, 1);

    let Some(recurrence) = projection.recurrence_curves.get("mem-cl-0001") else {
        return Err("landed lesson plus Store observation must emit a recurrence curve".into());
    };
    assert_eq!(recurrence.len(), 1);
    assert!(recurrence[0].since_landing);
    assert_eq!(recurrence[0].running_recurrence_count, 1);

    let Some(harness) = projection.learning_curves.get(&RecordDomain::Harness) else {
        return Err("fixture has landed harness lessons".into());
    };
    assert!(
        harness
            .iter()
            .any(|point| point.lesson_id == "mem-cl-0001" && point.cumulative_incidents >= 1),
        "Store-derived learning curve must include the replayed t0 observation"
    );
    Ok(())
}

/// Cross-cutting: the fail-closed evidence-chain contract and the
/// SessionStart recall pack both consume the same fixture graph
/// consistently -- exercised here so the acceptance surface's two
/// owner-set RESTORED items (usage-ingestion seam, `memory evidence
/// <lessonId>`) are proven end to end, not just unit-by-unit.
#[test]
fn evidence_chain_and_recall_pack_are_consistent_over_the_fixture(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = load_fixture_graph()?;
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-cl-0001".to_string(),
            rule_id: Some("CL-UNKNOWN-RULE".to_string()),
            fault_class: Some("unknown_rule_id".to_string()),
            repo_context: "crates/enforcer-scan".to_string(),
            clean: false,
            source_surface: "scan".to_string(),
            ts: "2026-07-05T10:12:00Z".to_string(),
        },
    );

    struct FixedLookup(HashMap<String, Vec<String>>);
    impl ProofRefLookup for FixedLookup {
        fn lookup(&self, landed_at_ref: &str) -> Vec<String> {
            self.0.get(landed_at_ref).cloned().unwrap_or_default()
        }
    }
    let mut refs = HashMap::new();
    refs.insert(
        "commit cl-0001-fix".to_string(),
        vec!["proof/journal/x06-cl-0001".to_string()],
    );
    let lookup = FixedLookup(refs);

    match evidence_chain(&graph, "mem-cl-0001", &lookup) {
        EvidenceReport::Chain {
            landed,
            has_t0_provenance,
            ..
        } => {
            assert!(has_t0_provenance, "mem-cl-0001 has an observed incident");
            assert_eq!(landed.len(), 1);
            assert_eq!(
                landed[0].proof_refs,
                vec!["proof/journal/x06-cl-0001".to_string()],
                "landed step must carry the enforcer-proof journal ref the caller's lookup found"
            );
        }
        EvidenceReport::Unknown { .. } => {
            return Err("mem-cl-0001 is a known, landed lesson".into())
        }
    }

    // An id with no node at all is fail-closed Unknown, never a
    // fabricated chain.
    assert!(matches!(
        evidence_chain(&graph, "mem-cl-does-not-exist", &NoProofRefs),
        EvidenceReport::Unknown { .. }
    ));

    let pack = recall_pack(&graph, 10);
    assert!(
        pack.active_lessons
            .iter()
            .any(|s| s.lesson_id == "mem-cl-0001"),
        "session-start recall pack must surface the active landed lesson"
    );
    assert!(
        !pack
            .active_lessons
            .iter()
            .any(|s| s.lesson_id == "mem-cl-0004"),
        "session-start recall pack must exclude the unlanded lesson"
    );
    Ok(())
}
