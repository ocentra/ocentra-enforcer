use enforcer_domain::memory_types::{FreshnessState, GraphEventKind, ProjectStatus};
use enforcer_memory::boundary::log_schema::{
    GraphEventLogEntryDto, ObservationLogEntryDto, SCHEMA_VERSION,
};
use enforcer_memory::error::MemoryError;
use enforcer_memory::projects::{
    delete_project, index_status, list_projects, LogIndexStatus, ProjectsError, ProjectsResult,
};
use enforcer_memory::store::manifest::write_index_manifest;
use enforcer_memory::store::sqlite::OperationalGraph;
use enforcer_memory::store::Store;
use std::path::PathBuf;

const STORE_MARKER_FILE: &str = "store.json";

fn temp_dir(name: &str) -> PathBuf {
    let unique = format!(
        "enforcer-memory-projects-{}-{}-{name}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

fn init_repo_root(raw: &str) -> ProjectsResult<enforcer_domain::paths::RepoRoot> {
    raw.parse().map_err(|source| {
        ProjectsError::Memory(MemoryError::InvalidPath {
            path: raw.to_owned().into(),
            source,
        })
    })
}

#[test]
fn list_projects_returns_empty_for_a_missing_stores_dir() -> ProjectsResult<()> {
    let stores_dir = temp_dir("missing");
    let projects = list_projects(&stores_dir)?;
    assert!(projects.is_empty());
    Ok(())
}

#[test]
fn list_projects_reports_every_initialized_project_and_skips_non_projects() -> ProjectsResult<()> {
    let stores_dir = temp_dir("list");
    let root_a = init_repo_root("C:/Projects/alpha")?;
    let root_b = init_repo_root("C:/Projects/beta")?;

    Store::init(&stores_dir, &root_a, "2026-07-05T00:00:00Z")?;
    Store::init(&stores_dir, &root_b, "2026-07-05T00:00:01Z")?;

    // An incidental non-project directory (no store.json) must be
    // skipped, not reported and not erroring the whole call.
    std::fs::create_dir_all(stores_dir.join("not-a-project")).map_err(|source| {
        MemoryError::Io {
            path: stores_dir.clone().into(),
            source,
        }
    })?;

    let projects = list_projects(&stores_dir)?;
    assert_eq!(projects.len(), 2);
    let mut repo_roots: Vec<&str> = projects.iter().map(|p| p.repo_root.as_str()).collect();
    repo_roots.sort_unstable();
    assert_eq!(repo_roots, vec!["C:/Projects/alpha", "C:/Projects/beta"]);

    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn index_status_reports_no_index_built_when_no_manifest_exists() -> ProjectsResult<()> {
    let stores_dir = temp_dir("status-fresh");
    let root = init_repo_root("C:/Projects/gamma")?;
    let store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
    let project_id = store.project_id().as_str().to_owned();

    let status = index_status(&stores_dir, &project_id)?;
    assert_eq!(status.project_id, project_id);
    assert_eq!(status.logs.len(), 2);
    assert!(status
        .logs
        .iter()
        .all(|l| matches!(l.state, FreshnessState::NoIndexBuilt)));
    // A brand-new project has no SQLite file yet -- baseline-aligned
    // (nodes>0 ? ready : empty) must report Empty, not error.
    assert_eq!(status.nodes, 0);
    assert_eq!(status.edges, 0);
    assert_eq!(status.status, ProjectStatus::Empty);

    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn index_status_reports_ready_once_the_operational_graph_has_nodes() -> ProjectsResult<()> {
    let stores_dir = temp_dir("status-ready");
    let root = init_repo_root("C:/Projects/zeta")?;
    let store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
    let project_id = store.project_id().as_str().to_owned();
    let sqlite_path = store.sqlite_path();
    drop(store);

    {
        let mut graph = OperationalGraph::open(&sqlite_path).map_err(ProjectsError::Memory)?;
        graph
            .apply(&GraphEventLogEntryDto {
                schema_version: SCHEMA_VERSION,
                seq: 0,
                id: "evt-0000".to_owned(),
                event: GraphEventKind::NodeAdded {
                    node_id: "file:lib.rs".into(),
                    node_kind: "file".into(),
                },
                ts: "2026-07-05T00:00:00Z".to_owned(),
                supersedes_seq: None,
            })
            .map_err(ProjectsError::Memory)?;
    }

    let status = index_status(&stores_dir, &project_id)?;
    assert_eq!(status.nodes, 1);
    assert_eq!(status.status, ProjectStatus::Ready);

    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn index_status_detects_a_stale_index_after_the_log_grows() -> ProjectsResult<()> {
    let stores_dir = temp_dir("status-stale");
    let root = init_repo_root("C:/Projects/delta")?;
    let mut store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
    let project_id = store.project_id().as_str().to_owned();
    let store_root = store.root().to_path_buf();

    // Append one entry so the log length advances past a manifest
    // that will be written for length 0.
    store
        .observation_log_mut()
        .append_with_seq(|seq| ObservationLogEntryDto {
            schema_version: SCHEMA_VERSION,
            seq: seq.into(),
            id: "obs-test-0000".to_owned(),
            lesson_id: "L1".to_owned(),
            rule_id: None,
            fault_class: None,
            repo_context: "crates/enforcer-memory".to_owned(),
            clean: true,
            source_surface: "test".to_owned(),
            ts: "2026-07-05T00:00:00Z".to_owned(),
            supersedes_seq: None,
            payload_kind: None,
            payload: None,
        })
        .map_err(ProjectsError::Memory)?;

    write_index_manifest(
        store_root.join("observations.index-manifest.json"),
        "observations",
        0,
        "2026-07-05T00:00:00Z",
    )
    .map_err(ProjectsError::Memory)?;

    let status = index_status(&stores_dir, &project_id)?;
    let observations: Vec<&LogIndexStatus> = status
        .logs
        .iter()
        .filter(|l| l.log_name == "observations")
        .collect();
    assert_eq!(observations.len(), 1, "observations log status present");
    assert!(matches!(
        observations[0].state,
        FreshnessState::Stale { .. }
    ));
    assert_eq!(observations[0].log_length, 1);

    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn delete_project_removes_only_the_derived_store_directory() -> ProjectsResult<()> {
    let stores_dir = temp_dir("delete-happy");
    let root = init_repo_root("C:/Projects/epsilon")?;
    let store = Store::init(&stores_dir, &root, "2026-07-05T00:00:00Z")?;
    let project_id = store.project_id().as_str().to_owned();
    let store_root = store.root().to_path_buf();
    drop(store);

    assert!(store_root.exists());
    delete_project(&stores_dir, &project_id)?;
    assert!(
        !store_root.exists(),
        "the project's own directory must be gone"
    );
    assert!(stores_dir.exists(), "stores_dir itself must survive");

    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn delete_project_rejects_an_unknown_project_id() -> ProjectsResult<()> {
    let stores_dir = temp_dir("delete-unknown");
    std::fs::create_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.clone().into(),
        source,
    })?;

    let outcome = delete_project(&stores_dir, "never-initialized");
    assert!(matches!(outcome, Err(ProjectsError::UnknownProject { .. })));

    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn delete_project_rejects_path_traversal_via_dotdot_project_id() -> ProjectsResult<()> {
    let parent = temp_dir("traversal-parent");
    let stores_dir = parent.join("stores");
    std::fs::create_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.clone().into(),
        source,
    })?;

    // Plant a directory OUTSIDE stores_dir that a `..`-laden
    // project_id could reach, and give it a store.json marker so it
    // would pass the "is this a real store" check if the
    // containment check were missing or buggy.
    let escape_target = parent.join("victim");
    std::fs::create_dir_all(&escape_target).map_err(|source| MemoryError::Io {
        path: escape_target.clone().into(),
        source,
    })?;
    std::fs::write(
        escape_target.join(STORE_MARKER_FILE),
        r#"{"schema_version":1,"project_id":"victim","repo_root":"C:/victim","initialized_at":"2026-07-05T00:00:00Z"}"#,
    )
    .map_err(|source| MemoryError::Io {
        path: escape_target.clone().into(),
        source,
    })?;

    let traversal_id = "../victim";
    let outcome = delete_project(&stores_dir, traversal_id);
    assert!(
        matches!(outcome, Err(ProjectsError::PathTraversal { .. })),
        "expected PathTraversal, got {outcome:?}"
    );
    assert!(
        escape_target.exists(),
        "the directory outside stores_dir must survive the rejected delete"
    );

    std::fs::remove_dir_all(&parent).map_err(|source| MemoryError::Io {
        path: parent.into(),
        source,
    })?;
    Ok(())
}

#[test]
fn delete_project_rejects_deleting_stores_dir_itself() -> ProjectsResult<()> {
    let stores_dir = temp_dir("delete-self");
    std::fs::create_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.clone().into(),
        source,
    })?;
    std::fs::write(
        stores_dir.join(STORE_MARKER_FILE),
        r#"{"schema_version":1,"project_id":"self","repo_root":"C:/self","initialized_at":"2026-07-05T00:00:00Z"}"#,
    )
    .map_err(|source| MemoryError::Io {
        path: stores_dir.clone().into(),
        source,
    })?;

    let outcome = delete_project(&stores_dir, ".");
    assert!(matches!(outcome, Err(ProjectsError::PathTraversal { .. })));
    assert!(stores_dir.exists());

    std::fs::remove_dir_all(&stores_dir).map_err(|source| MemoryError::Io {
        path: stores_dir.into(),
        source,
    })?;
    Ok(())
}
