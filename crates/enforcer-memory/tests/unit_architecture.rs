use enforcer_memory::analysis::clustering::{self, ClusteringResult};
use enforcer_memory::architecture::{
    build_overview, build_report, Aspect, EntryPointKind, LayerCategory,
};
use enforcer_memory::code_graph::{CodeGraph, Manifest};
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

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

#[test]
fn architecture_overview_groups_files_into_crate_sections() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::create_dir_all(dir.path().join("crates/foo/src"))?;
    fs::create_dir_all(dir.path().join("crates/bar/src"))?;
    fs::write(dir.path().join("crates/foo/src/lib.rs"), "fn a() {}\n")?;
    fs::write(dir.path().join("crates/bar/src/lib.rs"), "fn b() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![
        dir.path().join("crates/foo/src/lib.rs"),
        dir.path().join("crates/bar/src/lib.rs"),
    ];
    graph.index_repository(dir.path(), &files, &Manifest::default())?;

    let overview = build_overview(&graph, 10);
    let names: Vec<&str> = overview
        .sections()
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, vec!["crates/bar", "crates/foo"]);
    assert_eq!(overview.total_files_json(), json!(2));
    assert_eq!(overview.total_symbols_json(), json!(2));
    Ok(())
}

#[test]
fn architecture_overview_reports_language_composition_and_hotspots() -> TestResult<()> {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(dir.path().join("a.rs"), "fn caller() { helper(); }\n")?;
    fs::write(dir.path().join("b.rs"), "fn helper() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![dir.path().join("a.rs"), dir.path().join("b.rs")];
    graph.index_repository(dir.path(), &files, &Manifest::default())?;

    let overview = build_overview(&graph, 5);
    assert_eq!(overview.language_counts_json(), json!([["Rust", 2]]));
    assert!(
        !overview.hotspots().is_empty(),
        "expected at least one hotspot entry"
    );
    Ok(())
}

#[test]
fn empty_graph_produces_empty_overview_not_panic() {
    let graph = CodeGraph::new();
    let overview = build_overview(&graph, 5);
    assert!(overview.sections().is_empty());
    assert_eq!(overview.total_files_json(), json!(0));
    assert_eq!(overview.total_symbols_json(), json!(0));
}

/// A small multi-crate fixture used across the aspect tests below:
/// `crates/api` declares a route and imports `crates/core`;
/// `crates/core` has no dependencies -- so `api` depends on `core`,
/// never the reverse.
fn build_two_crate_fixture(dir: &Path) -> TestResult<CodeGraph> {
    init_repo(dir)?;
    fs::create_dir_all(dir.join("crates/core/src"))?;
    fs::create_dir_all(dir.join("crates/api/src"))?;
    fs::write(
        dir.join("crates/core/Cargo.toml"),
        "[package]\nname=\"core\"\n",
    )?;
    fs::write(dir.join("crates/core/src/lib.rs"), "pub fn load() {}\n")?;
    fs::write(
        dir.join("crates/api/Cargo.toml"),
        "[package]\nname=\"api\"\n",
    )?;
    fs::write(
        dir.join("crates/api/src/main.rs"),
        "use core::load;\nfn handle() { load(); }\n",
    )?;
    fs::write(
        dir.join("crates/api/src/routes.js"),
        "app.get(\"/status\", (req, res) => {});\n",
    )?;
    commit_all(dir, "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![
        dir.join("crates/core/Cargo.toml"),
        dir.join("crates/core/src/lib.rs"),
        dir.join("crates/api/Cargo.toml"),
        dir.join("crates/api/src/main.rs"),
        dir.join("crates/api/src/routes.js"),
    ];
    graph.index_repository(dir, &files, &Manifest::default())?;
    Ok(graph)
}

// --- hard test: every aspect returns its typed section ------------

#[test]
fn all_aspect_populates_every_typed_section() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::All], None, 10, 20);
    assert!(report.overview.is_some());
    assert!(report.structure.is_some());
    assert!(report.dependencies.is_some());
    assert!(report.routes.is_some());
    assert!(report.languages.is_some());
    assert!(report.packages.is_some());
    assert!(report.entry_points.is_some());
    assert!(report.hotspots.is_some());
    assert!(report.boundaries.is_some());
    assert!(report.layers.is_some());
    assert!(report.file_tree.is_some());
    assert!(report.clusters.is_some());
    Ok(())
}

#[test]
fn single_aspect_request_populates_only_that_section() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Routes], None, 10, 20);
    assert!(report.routes.is_some());
    assert!(report.overview.is_none());
    assert!(report.structure.is_none());
    assert!(report.dependencies.is_none());
    assert!(report.languages.is_none());
    assert!(report.packages.is_none());
    assert!(report.entry_points.is_none());
    assert!(report.hotspots.is_none());
    assert!(report.boundaries.is_none());
    assert!(report.layers.is_none());
    assert!(report.file_tree.is_none());
    assert!(report.clusters.is_none());
    Ok(())
}

// --- hard test: path prefix filter ---------------------------------

#[test]
fn path_prefix_filters_structure_and_languages_to_matching_files_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(
        &graph,
        &[Aspect::Structure, Aspect::Languages],
        Some("crates/core/"),
        10,
        20,
    );
    let structure = report.structure.ok_or("expected structure section")?;
    assert!(
        structure.iter().all(|s| s.name == "crates/core"),
        "expected only crates/core section under the crates/core/ prefix, got {structure:?}"
    );
    let languages = report.languages.ok_or("expected languages section")?;
    // Only core/src/lib.rs (Rust) + core/Cargo.toml (ConfigToml)
    // live under crates/core/.
    let total: usize = languages.iter().map(|(_, count)| *count).sum();
    assert_eq!(
        total, 2,
        "expected exactly 2 files under crates/core/, got {languages:?}"
    );
    Ok(())
}

#[test]
fn absolute_or_traversal_path_prefixes_fail_closed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    for invalid_prefix in ["/", "/crates/core", "../crates/core", "crates/../api"] {
        let report = build_report(&graph, &[Aspect::Overview], Some(invalid_prefix), 10, 20);
        let overview = report.overview.ok_or("expected overview section")?;
        assert_eq!(
            overview.total_files_json(),
            json!(0),
            "invalid prefix {invalid_prefix:?} must not widen the query"
        );
        assert_eq!(
            overview.total_symbols_json(),
            json!(0),
            "invalid prefix {invalid_prefix:?} must not expose symbols"
        );
    }
    Ok(())
}

// --- hard test: routes aspect ---------------------------------------

#[test]
fn routes_aspect_reports_the_declared_route() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Routes], None, 10, 20);
    let routes = report.routes.ok_or("expected routes section")?;
    assert!(routes.iter().any(|r| r.method == "GET"
        && r.path == "/status"
        && r.declared_in == "crates/api/src/routes.js"));
    Ok(())
}

// --- hard test: packages aspect --------------------------------------

#[test]
fn packages_aspect_detects_both_cargo_manifests() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Packages], None, 10, 20);
    let packages = report.packages.ok_or("expected packages section")?;
    assert_eq!(packages.len(), 2);
    let core_pkg = packages
        .iter()
        .find(|p| p.manifest_rel_path == "crates/core/Cargo.toml")
        .ok_or("expected a crates/core/Cargo.toml package section")?;
    assert!(core_pkg
        .member_rel_paths
        .iter()
        .any(|p| p == "crates/core/src/lib.rs"));
    Ok(())
}

#[test]
fn package_fan_counts_respect_the_requested_path_scope() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Packages], Some("crates/core/"), 10, 20);
    let packages = report.packages.ok_or("expected packages section")?;
    let core = packages
        .iter()
        .find(|package| package.manifest_rel_path == "crates/core/Cargo.toml")
        .ok_or("expected core package under scoped query")?;

    assert_eq!(core.fan_in, 0, "out-of-scope callers must not contribute");
    assert_eq!(core.fan_out, 0, "out-of-scope callees must not contribute");
    Ok(())
}

// --- hard test: entry_points aspect ----------------------------------

#[test]
fn entry_points_aspect_finds_main_and_lib_and_route_handler() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::EntryPoints], None, 10, 20);
    let entries = report.entry_points.ok_or("expected entry_points section")?;
    assert!(entries
        .iter()
        .any(|e| e.rel_path == "crates/api/src/main.rs" && e.kind == EntryPointKind::BinaryMain));
    assert!(entries
        .iter()
        .any(|e| e.rel_path == "crates/core/src/lib.rs" && e.kind == EntryPointKind::LibraryRoot));
    assert!(entries
        .iter()
        .any(|e| e.kind == EntryPointKind::RouteHandler));
    Ok(())
}

// --- hard test: dependencies + boundaries aspects ----------------------

#[test]
fn dependencies_aspect_reports_api_depends_on_core() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Dependencies], None, 10, 20);
    let deps = report.dependencies.ok_or("expected dependencies section")?;
    assert_eq!(deps.len(), 1, "expected one cross-section dependency");
    Ok(())
}

#[test]
fn boundaries_aspect_reports_the_undirected_pair() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Boundaries], None, 10, 20);
    let boundaries = report.boundaries.ok_or("expected boundaries section")?;
    // Boundaries is CALLS-only and directed (baseline-aligned):
    // api's main.rs calls core's load, so api -> core, never the
    // reverse.
    assert_eq!(boundaries.len(), 1, "expected one directed boundary");
    Ok(())
}

// --- hard test: layer ordering on an acyclic fixture --------------------

#[test]
fn layers_aspect_orders_core_before_api_on_acyclic_fixture() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Layers], None, 10, 20);
    let layering = report.layers.ok_or("expected layers section")?;
    assert!(
        layering.cycle_cluster_ids.is_empty(),
        "acyclic fixture must report no cycle cluster ids, got {:?}",
        layering.cycle_cluster_ids
    );
    assert!(!layering.layers.is_empty());

    // Find which layer contains the cluster with core's lib.rs vs
    // the cluster with api's main.rs, and assert core's layer index
    // is <= api's (core is depended upon, so it must not appear
    // strictly after api).
    let clusters_report = build_report(&graph, &[Aspect::Clusters], None, 10, 20);
    let clusters = clusters_report
        .clusters
        .ok_or("expected clusters section")?;

    let core_cluster = clusters
        .clusters
        .iter()
        .find(|c| {
            c.file_ids
                .iter()
                .any(|id| id == "file:crates/core/src/lib.rs")
        })
        .ok_or("expected a cluster containing core/src/lib.rs")?;
    let api_cluster = clusters
        .clusters
        .iter()
        .find(|c| {
            c.file_ids
                .iter()
                .any(|id| id == "file:crates/api/src/main.rs")
        })
        .ok_or("expected a cluster containing api/src/main.rs")?;

    let layer_of = |cluster_id: &str| -> Option<usize> {
        layering
            .layers
            .iter()
            .find(|l| l.cluster_ids.iter().any(|id| id == cluster_id))
            .map(|l| l.index)
    };

    if core_cluster.id != api_cluster.id {
        let core_layer = layer_of(&core_cluster.id).ok_or("core cluster missing from layers")?;
        let api_layer = layer_of(&api_cluster.id).ok_or("api cluster missing from layers")?;
        assert!(
            core_layer <= api_layer,
            "expected core's layer ({core_layer}) <= api's layer ({api_layer}) since api depends on core"
        );
    }
    Ok(())
}

// --- hard test: file_tree counts ----------------------------------------

#[test]
fn file_tree_reports_per_dir_counts_and_recursive_totals() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::FileTree], None, 10, 20);
    let tree = report.file_tree.ok_or("expected file_tree section")?;

    assert_eq!(tree.root.dir, ".");
    assert_eq!(
        tree.root.total_file_count, 5,
        "expected all 5 fixture files counted recursively from the root"
    );

    // Find the crates/api/src node and check its direct counts.
    let crates_node = tree
        .root
        .children
        .iter()
        .find(|c| c.dir == "crates")
        .ok_or("expected a crates/ node")?;
    let api_node = crates_node
        .children
        .iter()
        .find(|c| c.dir == "crates/api")
        .ok_or("expected a crates/api node")?;
    let api_src_node = api_node
        .children
        .iter()
        .find(|c| c.dir == "crates/api/src")
        .ok_or("expected a crates/api/src node")?;
    assert_eq!(
        api_src_node.direct_file_count, 2,
        "expected main.rs + routes.js directly under crates/api/src"
    );
    assert!(api_src_node.direct_symbol_count > 0);
    Ok(())
}

#[test]
fn empty_graph_file_tree_is_just_the_root_with_zero_counts() -> TestResult {
    let graph = CodeGraph::new();
    let report = build_report(&graph, &[Aspect::FileTree], None, 10, 20);
    let tree = report.file_tree.ok_or("expected file_tree section")?;
    assert_eq!(tree.root.dir, ".");
    assert_eq!(tree.root.total_file_count, 0);
    assert!(tree.root.children.is_empty());
    Ok(())
}

// --- hard test: clusters aspect wiring ----------------------------------

#[test]
fn clusters_aspect_returns_clustering_result() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Clusters], None, 10, 20);
    let clusters = report.clusters.ok_or("expected clusters section")?;
    assert!(!clusters.clusters.is_empty());
    Ok(())
}

// --- baseline-alignment hard tests (X06.P4 correction pass) ------------

#[test]
fn hotspot_entries_rank_by_calls_fan_in_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    // `hub` is called by two other functions; `lonely` is never
    // called -- fan_in must rank hub above lonely regardless of
    // hub's own outgoing call count (out-degree must not leak in).
    fs::write(dir.path().join("hub.rs"), "fn hub() {}\nfn lonely() {}\n")?;
    fs::write(dir.path().join("caller_a.rs"), "fn a() { hub(); }\n")?;
    fs::write(dir.path().join("caller_b.rs"), "fn b() { hub(); }\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![
        dir.path().join("hub.rs"),
        dir.path().join("caller_a.rs"),
        dir.path().join("caller_b.rs"),
    ];
    graph.index_repository(dir.path(), &files, &Manifest::default())?;

    let report = build_report(&graph, &[Aspect::Hotspots], None, 10, 20);
    let entries = report
        .hotspot_entries
        .ok_or("expected hotspot_entries section")?;
    let hub_entry = entries
        .iter()
        .find(|e| e.name == "hub")
        .ok_or("expected a hub entry")?;
    assert_eq!(
        hub_entry.fan_in, 2,
        "hub is called from 2 distinct call sites"
    );
    let lonely_entry = entries.iter().find(|e| e.name == "lonely");
    if let Some(lonely) = lonely_entry {
        assert_eq!(lonely.fan_in, 0);
    }
    // hub must rank above (or equal, but never below) lonely.
    let hub_pos = entries.iter().position(|e| e.name == "hub");
    let lonely_pos = entries.iter().position(|e| e.name == "lonely");
    if let (Some(hp), Some(lp)) = (hub_pos, lonely_pos) {
        assert!(hp < lp, "hub (fan_in=2) must rank above lonely (fan_in=0)");
    }
    Ok(())
}

#[test]
fn hotspot_entries_exclude_test_path_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::create_dir_all(dir.path().join("tests"))?;
    fs::write(
        dir.path().join("tests/helper_test.rs"),
        "fn tested_helper() {}\n",
    )?;
    fs::write(
        dir.path().join("caller.rs"),
        "fn caller() { tested_helper(); }\n",
    )?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    let files = vec![
        dir.path().join("tests/helper_test.rs"),
        dir.path().join("caller.rs"),
    ];
    graph.index_repository(dir.path(), &files, &Manifest::default())?;

    let report = build_report(&graph, &[Aspect::Hotspots], None, 10, 20);
    let entries = report
        .hotspot_entries
        .ok_or("expected hotspot_entries section")?;
    assert!(
        !entries.iter().any(|e| e.name == "tested_helper"),
        "a symbol declared under tests/ must be excluded from baseline-aligned hotspots, got {entries:?}"
    );
    Ok(())
}

#[test]
fn layer_classification_labels_api_section_as_api() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Layers], None, 10, 20);
    let classification = report
        .layer_classification
        .ok_or("expected layer_classification section")?;
    let api_entry = classification
        .iter()
        .find(|c| c.name == "crates/api")
        .ok_or("expected a crates/api classification entry")?;
    assert_eq!(api_entry.layer, LayerCategory::Api);
    assert!(!api_entry.reason.is_empty());
    Ok(())
}

#[test]
fn layer_classification_labels_isolated_section_as_internal() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    fs::write(dir.path().join("standalone.rs"), "fn solo() {}\n")?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(
        dir.path(),
        &[dir.path().join("standalone.rs")],
        &Manifest::default(),
    )?;

    let report = build_report(&graph, &[Aspect::Layers], None, 10, 20);
    let classification = report
        .layer_classification
        .ok_or("expected layer_classification section")?;
    assert!(classification
        .iter()
        .any(|c| c.name == "." && c.layer == LayerCategory::Internal));
    Ok(())
}

#[test]
fn package_sections_report_nonzero_fan_in_and_fan_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Packages], None, 10, 20);
    let packages = report.packages.ok_or("expected packages section")?;
    let api_pkg = packages
        .iter()
        .find(|p| p.manifest_rel_path == "crates/api/Cargo.toml")
        .ok_or("expected a crates/api/Cargo.toml package section")?;
    assert!(
        api_pkg.fan_out > 0,
        "api calls into core, so its package fan_out must be > 0, got {api_pkg:?}"
    );
    let core_pkg = packages
        .iter()
        .find(|p| p.manifest_rel_path == "crates/core/Cargo.toml")
        .ok_or("expected a crates/core/Cargo.toml package section")?;
    assert!(
        core_pkg.fan_in > 0,
        "core is called from api, so its package fan_in must be > 0, got {core_pkg:?}"
    );
    Ok(())
}

#[test]
fn cluster_cohesion_is_populated_and_bounded_zero_to_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let graph = build_two_crate_fixture(dir.path())?;

    let report = build_report(&graph, &[Aspect::Clusters], None, 10, 20);
    let cohesion = report
        .cluster_cohesion
        .ok_or("expected cluster_cohesion section")?;
    assert!(!cohesion.is_empty());
    for entry in &cohesion {
        assert!(
            (0.0..=1.0).contains(&entry.cohesion),
            "cohesion must be in [0.0, 1.0], got {} for cluster {}",
            entry.cohesion,
            entry.cluster_id
        );
    }
    // Ranked by descending member count.
    for i in 1..cohesion.len() {
        assert!(cohesion[i - 1].member_count >= cohesion[i].member_count);
    }
    Ok(())
}

#[test]
fn routes_aspect_is_capped_at_twenty() -> TestResult {
    let dir = tempfile::tempdir()?;
    init_repo(dir.path())?;
    let mut body = String::new();
    for i in 0..25 {
        body.push_str(&format!("app.get(\"/route{i}\", (req, res) => {{}});\n"));
    }
    fs::write(dir.path().join("routes.js"), &body)?;
    commit_all(dir.path(), "first")?;

    let mut graph = CodeGraph::new();
    graph.index_repository(
        dir.path(),
        &[dir.path().join("routes.js")],
        &Manifest::default(),
    )?;

    let report = build_report(&graph, &[Aspect::Routes], None, 10, 20);
    let routes = report.routes.ok_or("expected routes section")?;
    assert!(
        routes.len() <= 20,
        "routes aspect must be capped at 20 (baseline-aligned), got {}",
        routes.len()
    );
    Ok(())
}

#[test]
fn layers_aspect_reports_cycle_without_panicking() {
    // Two clusters whose only inter-cluster edges point at each
    // other in both directions -- a 2-cycle with no way to
    // establish a partial order.
    let clusters = ClusteringResult {
        clusters: vec![
            clustering::Cluster {
                id: "cluster-a".to_string(),
                member_node_ids: vec!["file:a.rs".to_string()],
                file_ids: vec!["file:a.rs".to_string()],
                symbol_ids: vec![],
            },
            clustering::Cluster {
                id: "cluster-b".to_string(),
                member_node_ids: vec!["file:b.rs".to_string()],
                file_ids: vec!["file:b.rs".to_string()],
                symbol_ids: vec![],
            },
        ],
        inter_cluster_edges: vec![
            clustering::InterClusterEdge {
                from_cluster: "cluster-a".to_string(),
                to_cluster: "cluster-b".to_string(),
                count: 1,
            },
            clustering::InterClusterEdge {
                from_cluster: "cluster-b".to_string(),
                to_cluster: "cluster-a".to_string(),
                count: 1,
            },
        ],
    };

    let result = enforcer_memory::architecture::layering(&clusters);
    assert!(
        result.layers.is_empty(),
        "a pure 2-cycle has no valid layer"
    );
    let mut cycle = result.cycle_cluster_ids;
    cycle.sort();
    assert_eq!(
        cycle,
        vec!["cluster-a".to_string(), "cluster-b".to_string()]
    );
}
