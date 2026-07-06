use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{ingest_observation, Observation};
use enforcer_memory::learning::{
    active_lessons, learning_curve, lesson_status, superseded_by, LessonStatus,
};
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::record::{MemoryRecord, Provenance, RecordDomain, RecordKind};

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
fn learning_curve_tracks_landed_count_and_incidents_per_domain() {
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
        None => unreachable!("harness domain has one landed lesson"),
    }

    match curves.get(&RecordDomain::Code) {
        Some(code) => {
            assert_eq!(
                code[0].cumulative_incidents, 0,
                "no incidents recorded for mem-a-0002"
            );
        }
        None => unreachable!("code domain has one landed lesson"),
    }
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
