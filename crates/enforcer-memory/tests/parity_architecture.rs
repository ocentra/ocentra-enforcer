//! Parity coverage for X06.P4 -- the `get_architecture` aspect surface,
//! aligned to `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! §7 (the ground-truth C-source wire-contract extraction; supersedes
//! the higher-level scout digest's "aspects incl. Leiden/Louvain
//! clustering, hotspots, layers, file_tree" summary this file
//! originally cited) -- over a small two-crate fixture repo copied from
//! `tests/fixtures/memory/architecture/`:
//!
//! - `crates/core/`: a `Cargo.toml` + `src/lib.rs` (`load`, `validate`)
//!   with no outgoing dependencies -- the upstream/depended-upon crate;
//! - `crates/api/`: a `Cargo.toml` + `src/main.rs` (imports `core`,
//!   calls both `load` and `validate`) + `src/routes.js` (declares a
//!   `GET /status` route) -- the downstream crate, so `api` depends on
//!   `core`, never the reverse.
//!
//! This fixture (unlike the flat fixtures under
//! `tests/fixtures/memory/{code_graph,graph_algorithms}/`) is nested on
//! purpose: the `path` prefix filter and the `Layers`/`Dependencies`/
//! `Boundaries` aspects are only meaningfully testable across more than
//! one directory, so this file's [`copy_fixtures_recursive`] walks
//! subdirectories -- the existing flat `copy_fixtures` helper other
//! X06 integration tests use does not need to change.

use enforcer_domain::memory_types::{Aspect, EntryPointKind, LayerCategory};
use enforcer_memory::architecture;
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/architecture";

fn run_git(dir: &Path, args: &[&str]) -> TestResult {
    let status = Command::new("git").args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(())
}

fn init_repo(dir: &Path) -> TestResult {
    run_git(dir, &["init", "--quiet"])?;
    run_git(dir, &["config", "user.email", "test@example.com"])?;
    run_git(dir, &["config", "user.name", "Test"])?;
    Ok(())
}

fn commit_all(dir: &Path, message: &str) -> TestResult {
    run_git(dir, &["add", "-A"])?;
    run_git(dir, &["commit", "--quiet", "-m", message])?;
    Ok(())
}

/// Recursively copy every file under `tests/fixtures/memory/architecture/`
/// into `dest`, preserving relative subdirectory structure, returning
/// every copied destination path (for `index_repository`'s `walk_files`
/// argument).
fn copy_fixtures_recursive(dest: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = manifest_dir.join(FIXTURE_DIR);
    let mut copied = Vec::new();
    copy_dir_recursive(&fixture_root, dest, &mut copied)?;
    Ok(copied)
}

fn copy_dir_recursive(
    src: &Path,
    dest: &Path,
    copied: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&dest_path)?;
            copy_dir_recursive(&entry.path(), &dest_path, copied)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &dest_path)?;
            copied.push(dest_path);
        }
    }
    Ok(())
}

fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    let files = copy_fixtures_recursive(dir)?;
    commit_all(dir, "initial fixture import")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

// --- hard test: every aspect returns its typed section -----------------

#[test]
fn every_named_aspect_returns_a_populated_typed_section() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    for aspect in [
        Aspect::Overview,
        Aspect::Structure,
        Aspect::Dependencies,
        Aspect::Routes,
        Aspect::Languages,
        Aspect::Packages,
        Aspect::EntryPoints,
        Aspect::Hotspots,
        Aspect::Boundaries,
        Aspect::Layers,
        Aspect::FileTree,
        Aspect::Clusters,
    ] {
        let report = architecture::build_report(&graph, &[aspect], None, 20, 30);
        let any_populated = report.overview.is_some()
            || report.structure.is_some()
            || report.dependencies.is_some()
            || report.routes.is_some()
            || report.languages.is_some()
            || report.packages.is_some()
            || report.entry_points.is_some()
            || report.hotspots.is_some()
            || report.boundaries.is_some()
            || report.layers.is_some()
            || report.file_tree.is_some()
            || report.clusters.is_some();
        assert!(
            any_populated,
            "aspect {aspect:?} produced an entirely empty report"
        );
    }
    Ok(())
}

#[test]
fn all_aspect_populates_every_section_at_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(&graph, &[Aspect::All], None, 20, 30);
    assert!(matches!(
        report,
        architecture::ArchitectureReport {
            overview: Some(_),
            structure: Some(_),
            dependencies: Some(_),
            routes: Some(_),
            languages: Some(_),
            packages: Some(_),
            entry_points: Some(_),
            hotspots: Some(_),
            hotspot_entries: Some(_),
            boundaries: Some(_),
            layers: Some(_),
            layer_classification: Some(_),
            file_tree: Some(_),
            clusters: Some(_),
            cluster_cohesion: Some(_),
        }
    ));
    Ok(())
}

// --- hard test: path filter ---------------------------------------------

#[test]
fn path_prefix_scopes_structure_to_the_matching_crate_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(
        &graph,
        &[Aspect::Structure],
        Some("crates/core/".into()),
        20,
        30,
    );
    let structure = report.structure.ok_or("expected structure section")?;
    assert!(
        !structure.is_empty(),
        "expected at least one section under the crates/core/ prefix"
    );
    assert!(
        structure.iter().all(|s| s.name == "crates/core"),
        "expected only crates/core to appear under its own path prefix, got {structure:?}"
    );

    let report_api = architecture::build_report(
        &graph,
        &[Aspect::Structure],
        Some("crates/api/".into()),
        20,
        30,
    );
    let structure_api = report_api.structure.ok_or("expected structure section")?;
    assert!(structure_api.iter().all(|s| s.name == "crates/api"));
    Ok(())
}

#[test]
fn path_scope_does_not_include_a_sibling_with_a_shared_text_prefix() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let core = dir.path().join("crates/core/src/lib.rs");
    let corex = dir.path().join("crates/corex/src/lib.rs");
    fs::create_dir_all(core.parent().ok_or("core source parent")?)?;
    fs::create_dir_all(corex.parent().ok_or("corex source parent")?)?;
    fs::write(&core, "pub fn core() {}\n")?;
    fs::write(&corex, "pub fn corex() {}\n")?;
    commit_all(dir.path(), "add sibling crates")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(dir.path(), &[core, corex], &Manifest::default())?;
    let report = architecture::build_report(
        &graph,
        &[Aspect::Structure],
        Some("crates/core".into()),
        20,
        30,
    );
    let structure = report.structure.ok_or("expected structure section")?;

    assert_eq!(structure.len(), 1, "{structure:?}");
    assert_eq!(structure[0].name, "crates/core");
    Ok(())
}

#[test]
fn path_prefix_scopes_legacy_hotspots_to_requested_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(
        &graph,
        &[Aspect::Overview, Aspect::Hotspots],
        Some("crates/core/".into()),
        20,
        30,
    );
    let overview = report.overview.ok_or("expected overview section")?;
    let hotspots = report.hotspots.ok_or("expected hotspots section")?;
    let expected = [
        "sym:crates/core/src/lib.rs:1:load",
        "file:crates/core/Cargo.toml",
        "file:crates/core/src/lib.rs",
        "sym:crates/core/Cargo.toml:1:[package]\nname = \"core\"\nversion = \"0.1.0\"\n",
        "sym:crates/core/Cargo.toml:1:package",
        "sym:crates/core/src/lib.rs:3:validate",
    ];
    let overview_ids: Vec<&str> = overview
        .hotspots()
        .iter()
        .map(|score| score.node_id.as_str())
        .collect();
    let hotspot_ids: Vec<&str> = hotspots
        .iter()
        .map(|score| score.node_id.as_str())
        .collect();
    assert_eq!(overview_ids, expected);
    assert_eq!(hotspot_ids, expected);
    Ok(())
}

// --- hard test: layer ordering on this acyclic fixture -------------------

#[test]
fn layers_place_core_at_or_before_api_on_the_acyclic_fixture() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report =
        architecture::build_report(&graph, &[Aspect::Layers, Aspect::Clusters], None, 20, 30);
    let layering = report.layers.ok_or("expected layers section")?;
    assert!(
        layering.cycle_cluster_ids.is_empty(),
        "the fixture's api->core dependency is acyclic; expected no cycle cluster ids, got {:?}",
        layering.cycle_cluster_ids
    );

    let clusters = report.clusters.ok_or("expected clusters section")?;
    let core_cluster = clusters
        .clusters
        .iter()
        .find(|c| {
            c.file_ids
                .iter()
                .any(|id| id == "file:crates/core/src/lib.rs")
        })
        .ok_or("expected a cluster containing crates/core/src/lib.rs")?;
    let api_cluster = clusters
        .clusters
        .iter()
        .find(|c| {
            c.file_ids
                .iter()
                .any(|id| id == "file:crates/api/src/main.rs")
        })
        .ok_or("expected a cluster containing crates/api/src/main.rs")?;

    if core_cluster.id != api_cluster.id {
        let layer_of = |cluster_id: &str| -> Option<usize> {
            layering
                .layers
                .iter()
                .find(|l| l.cluster_ids.iter().any(|id| id == cluster_id))
                .map(|l| usize::from(l.index))
        };
        let core_layer = layer_of(&core_cluster.id).ok_or("core cluster missing a layer")?;
        let api_layer = layer_of(&api_cluster.id).ok_or("api cluster missing a layer")?;
        assert!(
            core_layer <= api_layer,
            "expected core's layer ({core_layer}) <= api's layer ({api_layer})"
        );
    }
    Ok(())
}

// --- hard test: dependencies + boundaries reflect the real direction ----

#[test]
fn dependencies_aspect_shows_api_depends_on_core_not_the_reverse() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(&graph, &[Aspect::Dependencies], None, 20, 30);
    let deps = report.dependencies.ok_or("expected dependencies section")?;
    assert_eq!(deps.len(), 1, "expected one directed dependency");
    Ok(())
}

#[test]
fn boundaries_aspect_reports_the_core_api_coupling() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(&graph, &[Aspect::Boundaries], None, 20, 30);
    let boundaries = report.boundaries.ok_or("expected boundaries section")?;
    // Baseline-aligned shape: directed, CALLS-only (`{from, to,
    // call_count}`) -- api's main.rs calls core's load/validate, so
    // api -> core, never the reverse.
    assert_eq!(boundaries.len(), 1, "expected one directed boundary");
    Ok(())
}

// --- hard test: file_tree counts ------------------------------------------

#[test]
fn file_tree_counts_match_the_fixture_layout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(&graph, &[Aspect::FileTree], None, 20, 30);
    let tree = report.file_tree.ok_or("expected file_tree section")?;
    assert_eq!(tree.root.dir, ".");
    assert_eq!(
        tree.root.total_file_count, 5,
        "expected all 5 fixture files (2 Cargo.toml + lib.rs + main.rs + routes.js) counted from the root"
    );

    let crates_node = tree
        .root
        .children
        .iter()
        .find(|c| c.dir == "crates")
        .ok_or("expected a crates/ node")?;
    assert_eq!(
        crates_node.children.len(),
        2,
        "expected api/ and core/ under crates/"
    );

    let api_src = crates_node
        .children
        .iter()
        .find(|c| c.dir == "crates/api")
        .ok_or("expected crates/api node")?
        .children
        .iter()
        .find(|c| c.dir == "crates/api/src")
        .ok_or("expected crates/api/src node")?
        .clone();
    assert_eq!(
        api_src.direct_file_count, 2,
        "expected main.rs + routes.js directly under crates/api/src"
    );
    assert!(api_src.direct_symbol_count > 0);
    Ok(())
}

// --- hard test: routes + entry_points + packages -------------------------

#[test]
fn routes_entry_points_and_packages_are_all_detected() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(
        &graph,
        &[Aspect::Routes, Aspect::EntryPoints, Aspect::Packages],
        None,
        20,
        30,
    );

    let routes = report.routes.ok_or("expected routes section")?;
    assert!(routes.iter().any(|r| r.method == "GET"
        && r.path == "/status"
        && r.declared_in == "crates/api/src/routes.js"));

    let entry_points = report.entry_points.ok_or("expected entry_points section")?;
    assert!(entry_points
        .iter()
        .any(|e| e.rel_path == "crates/api/src/main.rs" && e.kind == EntryPointKind::BinaryMain));
    assert!(entry_points
        .iter()
        .any(|e| e.rel_path == "crates/core/src/lib.rs" && e.kind == EntryPointKind::LibraryRoot));
    assert!(entry_points
        .iter()
        .any(|e| e.kind == EntryPointKind::RouteHandler));

    let packages = report.packages.ok_or("expected packages section")?;
    assert_eq!(packages.len(), 2);
    assert!(packages
        .iter()
        .any(|p| p.manifest_rel_path == "crates/core/Cargo.toml"));
    assert!(packages
        .iter()
        .any(|p| p.manifest_rel_path == "crates/api/Cargo.toml"));

    // Baseline schema doc §7.2: packages carry fan_in/fan_out (the
    // baseline's own values are documented as always-0/likely-stub;
    // this crate computes real CALLS-only cross-package counts).
    let core_pkg = packages
        .iter()
        .find(|p| p.manifest_rel_path == "crates/core/Cargo.toml")
        .ok_or("expected core package")?;
    assert!(core_pkg.fan_in > 0, "core is called into from api");
    let api_pkg = packages
        .iter()
        .find(|p| p.manifest_rel_path == "crates/api/Cargo.toml")
        .ok_or("expected api package")?;
    assert!(api_pkg.fan_out > 0, "api calls into core");
    Ok(())
}

// --- hard test: baseline-aligned hotspots (CALLS fan_in only) -----------

#[test]
fn hotspot_entries_use_calls_fan_in_not_total_degree() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(&graph, &[Aspect::Hotspots], None, 20, 30);
    // Both shapes present at once: back-compat total-degree AND the
    // new baseline-aligned fan-in-only ranking.
    let _hotspots = report.hotspots.ok_or("expected hotspots section")?;
    let entries = report
        .hotspot_entries
        .ok_or("expected hotspot_entries section")?;
    // core's `load` is called from api's main.rs -- fan_in >= 1.
    let load = entries
        .iter()
        .find(|entry| entry.name == "load")
        .ok_or("expected load in hotspot entries")?;
    assert!(
        load.fan_in >= 1,
        "load should have at least one CALLS in-edge"
    );
    Ok(())
}

// --- hard test: baseline-aligned layer classification -------------------

#[test]
fn layer_classification_categorizes_the_fixture_sections() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(&graph, &[Aspect::Layers], None, 20, 30);
    let classification = report
        .layer_classification
        .ok_or("expected layer_classification section")?;
    // api declares a route -> classified Api regardless of its
    // fan_in/fan_out standing.
    let api = classification
        .iter()
        .find(|c| c.name == "crates/api")
        .ok_or("expected a crates/api classification")?;
    assert_eq!(api.layer, LayerCategory::Api);
    Ok(())
}

// --- hard test: baseline-aligned cluster cohesion ------------------------

#[test]
fn cluster_cohesion_accompanies_the_clusters_aspect() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let report = architecture::build_report(&graph, &[Aspect::Clusters], None, 20, 30);
    let cohesion = report
        .cluster_cohesion
        .ok_or("expected cluster_cohesion section")?;
    assert!(cohesion
        .iter()
        .any(|entry| (0.0..=1.0).contains(&entry.cohesion)));
    for entry in &cohesion {
        assert!((0.0..=1.0).contains(&entry.cohesion));
    }
    Ok(())
}

// --- hard test: clustering determinism across two runs (same input) -----

#[test]
fn clustering_via_build_report_is_deterministic_across_two_runs() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_fixture_graph(dir.path())?;

    let run1 = architecture::build_report(&graph, &[Aspect::Clusters], None, 20, 30);
    let run2 = architecture::build_report(&graph, &[Aspect::Clusters], None, 20, 30);
    assert_eq!(run1.clusters, run2.clusters);
    Ok(())
}

#[test]
fn empty_graph_every_aspect_returns_empty_sections_not_panic() -> TestResult {
    let graph = CodeGraph::new();
    let report = architecture::build_report(&graph, &[Aspect::All], None, 20, 30);
    let overview = report
        .overview
        .ok_or("overview should be Some for an empty graph too")?;
    assert_eq!(overview.total_files_json(), serde_json::json!(0));
    let tree = report
        .file_tree
        .ok_or("file_tree should be Some for an empty graph too")?;
    assert_eq!(tree.root.total_file_count, 0);
    Ok(())
}
