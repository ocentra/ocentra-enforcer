use enforcer_domain::paths::RepoRoot;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{ingest_observation, ingest_observation_into_store, Observation};
use enforcer_memory::learning::{
    active_lessons, learning_curve, lesson_status, project_learning_from_store, superseded_by,
    LessonStatus,
};
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::model_observations::{
    record_model_runtime_observation_in_store, ModelRuntimeObservationCandidate,
    ModelRuntimeObservationRecord, RecurrenceNegativeKind, RecurrenceOrNegativeEvidence,
};
use enforcer_memory::observations::{
    record_procedural_in_store, record_route_choice_in_store, ProceduralOutcome,
    ProceduralStoreInput, RouteChoiceStoreInput,
};
use enforcer_memory::record::{MemoryRecordDto as MemoryRecord, Provenance, RecordDomain, RecordKind};
use enforcer_memory::store::Store;

fn record(id: &str, domain: RecordDomain, landed_at: Vec<&str>) -> MemoryRecord {
    MemoryRecord {
        schema_version: 1,
        id: id.to_string(),
        ts: "2026-07-04T00:00:00Z".to_string(),
        kind: RecordKind::Lesson,
        domain,
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

#[test]
fn landed_record_is_active_unlanded_is_inactive() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(record(
        "mem-a-0001",
        RecordDomain::Harness,
        vec!["commit abc"],
    ));
    graph.ingest_record(record("mem-a-0002", RecordDomain::Harness, vec![]));
    assert_eq!(
        lesson_status(&graph, "mem-a-0001"),
        Some(LessonStatus::Active)
    );
    assert_eq!(
        lesson_status(&graph, "mem-a-0002"),
        Some(LessonStatus::Inactive)
    );
    assert_eq!(lesson_status(&graph, "mem-a-nonexistent"), None);
}

#[test]
fn ledger_row_landed_at_drives_activation_too() {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "L1".to_string(),
        date: "2026-07-04".to_string(),
        observed: "x".to_string(),
        lesson: "y".to_string(),
        landed_at: "arc-16 finding".to_string(),
        ships_via: "arc-16".to_string(),
    });
    assert_eq!(lesson_status(&graph, "L1"), Some(LessonStatus::Active));
}

#[test]
fn active_lessons_excludes_inactive_and_superseded() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(record(
        "mem-a-0001",
        RecordDomain::Harness,
        vec!["commit abc"],
    ));
    graph.ingest_record(record("mem-a-0002", RecordDomain::Harness, vec![]));
    let mut superseder = record("mem-a-0003", RecordDomain::Harness, vec!["commit def"]);
    superseder.supersedes = Some("mem-a-0001".to_string());
    graph.ingest_record(superseder);

    let active = active_lessons(&graph);
    assert!(
        !active.contains(&"mem-a-0001"),
        "superseded, must be excluded"
    );
    assert!(
        !active.contains(&"mem-a-0002"),
        "unlanded, must be excluded"
    );
    assert!(active.contains(&"mem-a-0003"), "supersedes and is landed");
    assert_eq!(superseded_by(&graph, "mem-a-0001"), Some("mem-a-0003"));
    assert_eq!(superseded_by(&graph, "mem-a-0003"), None);
}

#[test]
fn learning_curve_tracks_landed_count_and_incidents_per_domain(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(record(
        "mem-a-0001",
        RecordDomain::Harness,
        vec!["commit abc"],
    ));
    graph.ingest_record(record("mem-a-0002", RecordDomain::Code, vec!["commit def"]));
    graph.ingest_record(record("mem-a-0003", RecordDomain::Harness, vec![]));

    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-a-0001".to_string(),
            rule_id: None,
            fault_class: None,
            repo_context: "crates/enforcer-memory".to_string(),
            clean: false,
            source_surface: "scan".to_string(),
            ts: "2026-07-04T01:00:00Z".to_string(),
        },
    );

    let curves = learning_curve(&graph);
    match curves.get(&RecordDomain::Harness) {
        Some(harness) => {
            assert_eq!(harness.len(), 1, "unlanded mem-a-0003 must not appear");
            assert_eq!(harness[0].lesson_id, "mem-a-0001");
            assert_eq!(harness[0].landed_count, 1);
            assert_eq!(harness[0].cumulative_incidents, 1);
        }
        None => return Err("harness domain has one landed lesson".into()),
    }

    match curves.get(&RecordDomain::Code) {
        Some(code) => {
            assert_eq!(
                code[0].cumulative_incidents, 0,
                "no incidents recorded for mem-a-0002"
            );
        }
        None => return Err("code domain has one landed lesson".into()),
    }
    Ok(())
}

#[test]
fn learning_curve_omits_domains_with_no_landed_lessons() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(record("mem-a-0001", RecordDomain::User, vec![]));
    let curves = learning_curve(&graph);
    assert!(
        !curves.contains_key(&RecordDomain::User),
        "no landed lesson in this domain -- must not fabricate an empty-but-present curve"
    );
}

#[test]
fn store_learning_projection_replays_observations_into_curves(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-learning-store".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-07T00:00:00Z")?;

    let mut seed_graph = MemoryGraph::new();
    seed_graph.ingest_record(record(
        "mem-a-0001",
        RecordDomain::Harness,
        vec!["commit abc"],
    ));

    let mut write_projection = MemoryGraph::new();
    ingest_observation_into_store(
        &mut store,
        &mut write_projection,
        Observation {
            lesson_id: "mem-a-0001".to_string(),
            rule_id: Some("LRN-STORE".to_string()),
            fault_class: Some("store-backed-recurrence".to_string()),
            repo_context: "crates/enforcer-memory/src/learning.rs".to_string(),
            clean: false,
            source_surface: "scan".to_string(),
            ts: "2026-07-07T00:01:00Z".to_string(),
        },
    )?;
    record_model_runtime_observation_in_store(
        &mut store,
        &ModelRuntimeObservationRecord::new(
            "2026-07-07T00:02:00Z",
            "model-runtime-proof",
            "run-lrn-store",
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidence(
                RecurrenceOrNegativeEvidence {
                    lesson_id: "mem-a-0001".to_string(),
                    query_id: Some("qa-lrn-store".to_string()),
                    evidence_kind: RecurrenceNegativeKind::RecurrenceCount {
                        recurrence_count: 2,
                        previous_count: Some(1),
                    },
                    clean_evidence: true,
                },
            ),
        ),
    )?;
    record_procedural_in_store(
        &mut store,
        &mut write_projection,
        &ProceduralStoreInput::new(
            "mem-a-0001",
            ProceduralOutcome::FixSuccess,
            "replayed store-backed recurrence projection",
            "2026-07-07T00:03:00Z",
        ),
    )?;
    record_route_choice_in_store(
        &mut store,
        &mut write_projection,
        &RouteChoiceStoreInput::new(
            "store-backed recurrence curve",
            "learning-projection",
            0.92,
            "2026-07-07T00:04:00Z",
        ),
    )?;

    assert!(
        seed_graph.incidents_for_lesson("mem-a-0001").is_empty(),
        "projection must not mutate the seed lesson graph"
    );

    let projection = project_learning_from_store(&store, &seed_graph)?;
    assert_eq!(projection.replayed_incident_observations, 2);
    assert_eq!(projection.replayed_procedural_and_routes, 2);
    assert_eq!(projection.model_runtime_observations, 1);
    assert_eq!(projection.procedural_record_count, 1);
    assert_eq!(projection.route_trace_count, 1);

    let Some(harness) = projection.learning_curves.get(&RecordDomain::Harness) else {
        return Err("landed harness lesson must produce a curve".into());
    };
    assert_eq!(harness.len(), 1);
    assert_eq!(harness[0].lesson_id, "mem-a-0001");
    assert_eq!(
        harness[0].cumulative_incidents, 2,
        "normal and model-runtime observations must both replay into the Store-derived curve"
    );

    let Some(recurrence) = projection.recurrence_curves.get("mem-a-0001") else {
        return Err("Store-derived recurrence curve must exist for the observed lesson".into());
    };
    assert_eq!(recurrence.len(), 2);
    assert!(recurrence.iter().all(|point| point.since_landing));
    assert_eq!(recurrence[1].running_recurrence_count, 2);
    Ok(())
}
