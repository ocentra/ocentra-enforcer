//! X06.3 + X06.P4: architecture overview / repo mind map.
//!
//! Answers the "architecture overview" and "repo mind map" hard
//! requirements, extended (X06.P4, the wave-3 parity push) to the full
//! `get_architecture` aspect surface. Aspect naming/count and every
//! per-aspect response shape below are aligned to
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! §7 (the ground-truth C-source extraction that landed after this
//! module's first pass and superseded the higher-level scout digest's
//! summary) -- **semantics, not SQL**: enforcer-memory re-derives every
//! metric from its own graph model rather than porting the baseline's C
//! queries, per MEMORY_RETRIEVAL_BORROW_POLICY (behavioral parity only,
//! never code, for the C baseline).
//!
//! - [`Aspect::All`] -- every aspect below, populated together (baseline
//!   §7.2: a meta-token, not its own response key);
//! - [`Aspect::Overview`] -- crate/module map + hotspots + language
//!   composition (the original X06.3 slice, unchanged shape; baseline
//!   §7.2: "all aspects except file_tree" -- a meta-token there, but
//!   this crate keeps it as its own typed section since Rust callers
//!   need a concrete return type regardless);
//! - [`Aspect::Structure`] -- the crate/module sections alone;
//! - [`Aspect::Dependencies`] -- directed crate-section -> crate-section
//!   edges, derived from resolved import/call edges (broader than
//!   [`Aspect::Boundaries`], which is CALLS-only per baseline §7.2);
//! - [`Aspect::Routes`] -- every declared HTTP-style route, capped at 20
//!   (baseline §7.2: `routes` capped at 20);
//! - [`Aspect::Languages`] -- language composition counts;
//! - [`Aspect::Packages`] -- detected package manifests (`Cargo.toml`,
//!   `package.json`), the files under each, and CALLS-only
//!   `fan_in`/`fan_out` (baseline §7.2 documents its own `fan_in`/
//!   `fan_out` as "always 0... UNVERIFIED why" -- a likely stub this
//!   crate does not reproduce; real counts are computed instead);
//! - [`Aspect::EntryPoints`] -- binary/library entry files (`main.rs`,
//!   `lib.rs`) plus route-declaring files;
//! - [`Aspect::Hotspots`] -- BOTH the original X06.3 total-degree metric
//!   ([`ArchitectureOverview::hotspots`]/[`crate::analysis::HotspotScore`],
//!   kept for back-compat) AND a baseline-aligned CALLS-fan-in-only
//!   ranking over Function/Test symbols with test files excluded
//!   ([`HotspotEntry`], baseline §7.3's exact SQL semantics re-derived);
//! - [`Aspect::Boundaries`] -- directed, CALLS-only cross-section edge
//!   counts, `{from, to, call_count}` (baseline §7.2's exact shape --
//!   this module's first pass had this aspect wrong as an undirected,
//!   import+call-mixed pair; corrected against the landed baseline
//!   schema doc);
//! - [`Aspect::Layers`] -- BOTH a topological ordering of
//!   [`crate::analysis::clustering`] communities by dependency
//!   direction with cycles reported rather than panicking (this pack's
//!   own hard-test requirement; the baseline classifier has no
//!   cycle-detection concept at all) AND a baseline-aligned rule-based
//!   `{name, layer, reason}` classification into `entry|api|core|leaf|
//!   internal` categories ([`LayerClassification`], baseline §7.2);
//! - [`Aspect::FileTree`] -- a hierarchical directory tree with
//!   per-directory file/symbol counts (baseline §7.2 returns a *flat*
//!   `[{path,type,children}]` array instead; this module keeps the
//!   richer nested shape this pack's own hard test explicitly requires
//!   -- OWNER_INTENT's "equal or better quality" clause, not a gap);
//! - [`Aspect::Clusters`] -- the de-facto module communities themselves
//!   ([`crate::analysis::clustering::detect_clusters`], deterministic
//!   label propagation, never Leiden's randomized tie-breaks) plus
//!   baseline-aligned per-cluster `cohesion`
//!   ([`ClusterCohesion`], baseline §7.4's exact formula). Baseline
//!   drops clusters with fewer than 2 members entirely; this crate's
//!   own clustering hard test requires singleton clusters to still
//!   appear (never silently drop data), so singletons are kept with a
//!   defined cohesion of `1.0` rather than dropped.
//!
//! [`build_report`] is the aspect-driven entry point; [`build_overview`]
//! (the original X06.3 function) is kept unchanged for existing callers
//! and is exactly [`build_report`] with `aspects = [Aspect::Overview]`'s
//! data, re-shaped to its original return type.
//!
//! Every section honors an optional `path` prefix filter: when set,
//! only files/symbols whose repo-relative path starts with the prefix
//! contribute to any requested section (baseline §7.1: `path` is a
//! "directory-prefix scope, applied uniformly across every requested
//! aspect").

use crate::analysis::clustering::{self, ClusteringResult};
use crate::analysis::CodeAdjacency;
use crate::code_graph::CodeGraph;
use std::collections::{BTreeMap, BTreeSet};

/// One crate/top-level-directory section of the architecture overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateSection {
    /// The top-level path segment (e.g. `crates/enforcer-memory` or the
    /// crate name if the repo root itself contains `Cargo.toml` files
    /// one level down) this section groups.
    pub name: String,
    pub file_count: usize,
    pub symbol_count: usize,
    pub rel_paths: Vec<String>,
}

/// The full architecture overview: crate map + hotspots + language
/// composition. Constructed by [`build_overview`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitectureOverview {
    pub sections: Vec<CrateSection>,
    pub hotspots: Vec<crate::analysis::HotspotScore>,
    pub language_counts: Vec<(String, usize)>,
    pub total_files: usize,
    pub total_symbols: usize,
}

/// Build the architecture overview for `graph`. `hotspot_limit` bounds
/// how many top hotspot entries are retained (the workpack does not
/// mandate a specific number; callers pick per their MCP/CLI surface).
pub fn build_overview(graph: &CodeGraph, hotspot_limit: usize) -> ArchitectureOverview {
    let sections = crate_sections(graph, None);
    let language_counts = language_counts(graph, None);
    let adjacency = CodeAdjacency::build(graph);
    let hotspots = adjacency.hotspots(hotspot_limit);
    let total_files = graph
        .file_nodes()
        .filter(|f| path_matches(&f.rel_path, None))
        .count();
    let total_symbols = symbol_count_under(graph, None);

    ArchitectureOverview {
        sections,
        hotspots,
        language_counts,
        total_files,
        total_symbols,
    }
}

// ---------------------------------------------------------------------
// X06.P4: full aspect surface
// ---------------------------------------------------------------------

/// Which `get_architecture` aspect(s) a caller wants. `All` expands to
/// every other variant at request time ([`build_report`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Aspect {
    All,
    Overview,
    Structure,
    Dependencies,
    Routes,
    Languages,
    Packages,
    EntryPoints,
    Hotspots,
    Boundaries,
    Layers,
    FileTree,
    Clusters,
}

/// One route entry (a re-shaping of [`crate::code_graph::RouteEdge`]
/// with the declaring file's path attached, since the raw edge only
/// carries the file *id*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntry {
    pub method: String,
    pub path: String,
    pub declared_in: String,
    pub line: usize,
}

/// One detected package manifest (`Cargo.toml`/`package.json`) and the
/// files that live under its directory. `fan_in`/`fan_out` are
/// baseline-aligned (baseline schema doc §7.2: `packages` ->
/// `[{name, node_count, fan_in, fan_out}]`) -- CALLS-only,
/// cross-package edge counts, computed here rather than left at zero
/// (the baseline doc flags its own `fan_in`/`fan_out` as "always 0...
/// UNVERIFIED why," i.e. likely a stub/bug, not a documented design
/// ceiling worth reproducing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSection {
    /// The manifest's own containing directory, `"."` for a root-level
    /// manifest.
    pub dir: String,
    pub manifest_rel_path: String,
    pub member_file_count: usize,
    pub member_rel_paths: Vec<String>,
    /// Incoming CALLS edges from outside this package's own directory.
    pub fan_in: usize,
    /// Outgoing CALLS edges to outside this package's own directory.
    pub fan_out: usize,
}

/// One entry-point file (`main.rs`, `lib.rs`, or any file declaring at
/// least one route -- the two shapes of "where execution begins" this
/// crate's graph can currently see).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPoint {
    pub rel_path: String,
    pub kind: EntryPointKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPointKind {
    BinaryMain,
    LibraryRoot,
    RouteHandler,
}

/// A directed dependency edge between two crate/module sections (see
/// [`CrateSection::name`]), with the number of resolved import/call
/// edges crossing from `from` into `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub count: usize,
}

/// A directed cross-section CALLS-edge count: `from` section calls into
/// `to` section `call_count` times. Baseline-aligned shape (baseline
/// tool schema doc §7.2: `boundaries` -> `[{from, to, call_count}]`,
/// "cross-package CALLS edge counts") -- directed, CALLS-only, distinct
/// from [`Aspect::Dependencies`]'s broader import+call edge set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary {
    pub from: String,
    pub to: String,
    pub call_count: usize,
}

/// One layer in the dependency-direction topological ordering of
/// [`crate::analysis::clustering`] communities: every cluster id in
/// this layer depends only on clusters in strictly earlier layers (or
/// on nothing). Layer 0 is the most upstream (depended-upon-by-everyone)
/// end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub index: usize,
    pub cluster_ids: Vec<String>,
}

/// The result of [`layering`]: either a clean topological ordering, or
/// -- for a graph with a dependency cycle -- the layers that *could* be
/// ordered plus the cycle's own cluster ids, reported rather than
/// panicking (the pack's "cycles reported, not panicked" hard
/// requirement).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayeringResult {
    pub layers: Vec<Layer>,
    /// Cluster ids that could not be placed into any layer because they
    /// participate in a dependency cycle. Empty for an acyclic graph.
    pub cycle_cluster_ids: Vec<String>,
}

/// One baseline-aligned hotspot entry (baseline tool schema doc §7.3:
/// exact SQL is `COUNT(*) fan_in` over CALLS in-edges into
/// Function/Method nodes, test files excluded, ranked by `fan_in`
/// descending with array position as the implicit rank -- no separate
/// score field). This is a *different* metric from
/// [`ArchitectureOverview::hotspots`]/[`crate::analysis::HotspotScore`]
/// (total in+out degree over every node kind, unchanged for existing
/// callers per this module's own back-compat contract) -- `Hotspots`
/// the aspect specifically matches the baseline's fan-in-only,
/// symbol-scoped, test-excluded semantics per the baseline schema doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotspotEntry {
    pub name: String,
    /// This crate's node id (`sym:<rel_path>:<line>:<name>`), the
    /// closest equivalent to baseline's `qualified_name`.
    pub node_id: String,
    pub fan_in: usize,
}

/// A file counts as a "test file" for hotspot exclusion when its path
/// contains a `test`/`tests`/`spec` segment or a `_test`/`.test`
/// filename marker -- the same class of heuristic the baseline schema
/// doc documents for its own hotspot SQL (`file_path NOT LIKE
/// '%test%'`) and for `trace_path`'s `include_tests` filter (§5.4),
/// re-expressed here rather than copied (BORROW_POLICY: behavior spec
/// only, no C code exists to copy).
fn looks_like_test_path(rel_path: &str) -> bool {
    let lower = rel_path.to_lowercase();
    lower.contains("test") || lower.contains("/spec/") || lower.contains("_spec.")
}

/// Baseline-aligned `hotspots` aspect: CALLS in-degree only, over
/// Function/Test symbol nodes (this crate's closest equivalent to the
/// baseline's Function/Method label pair), excluding symbols whose
/// declaring file looks like a test file, ranked descending by
/// `fan_in`, capped at `limit`. Ties broken by node id for determinism
/// (the baseline's `qsort` documents no tie-break at all -- this crate
/// never leaves an ordering undefined).
fn hotspot_entries(graph: &CodeGraph, prefix: Option<&str>, limit: usize) -> Vec<HotspotEntry> {
    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_names: Vec<(&str, &str)> = graph
        .symbol_nodes()
        .map(|s| (s.name.as_str(), s.id.as_str()))
        .collect();

    let is_function_like: BTreeSet<&str> = graph
        .nodes()
        .iter()
        .filter(|n| {
            matches!(
                n,
                crate::code_graph::CodeNode::Function(_) | crate::code_graph::CodeNode::Test(_)
            )
        })
        .map(|n| n.id())
        .collect();

    let mut fan_in: BTreeMap<&str, usize> = BTreeMap::new();
    for call in graph.calls() {
        let Some(to_symbol_id) = resolve_callee(&call.callee, &symbol_names) else {
            continue;
        };
        if !is_function_like.contains(to_symbol_id) {
            continue;
        }
        *fan_in.entry(to_symbol_id).or_insert(0) += 1;
    }

    let mut entries: Vec<HotspotEntry> = graph
        .symbol_nodes()
        .filter(|s| is_function_like.contains(s.id.as_str()))
        .filter_map(|s| {
            // `s.file_id` (e.g. `file:tests/helper_test.rs`) IS the
            // file node id already -- look it up directly in
            // `file_path_by_id`, never through a symbol-id-keyed map
            // (there is no such intermediate step; a symbol belongs to
            // exactly one file, recorded on the symbol itself).
            let file_path = file_path_by_id
                .get(s.file_id.as_str())
                .copied()
                .unwrap_or("");
            if !path_matches(file_path, prefix) || looks_like_test_path(file_path) {
                return None;
            }
            Some(HotspotEntry {
                name: s.name.clone(),
                node_id: s.id.clone(),
                fan_in: fan_in.get(s.id.as_str()).copied().unwrap_or(0),
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        b.fan_in
            .cmp(&a.fan_in)
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    entries.truncate(limit);
    entries
}

/// One directory's entry in [`FileTree`]: its own file/symbol counts
/// (direct children only) plus nested subdirectories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTreeNode {
    /// Repo-relative directory path, `"."` for the repo root.
    pub dir: String,
    /// Files directly in this directory (not in a subdirectory).
    pub direct_file_count: usize,
    /// Symbols belonging to files directly in this directory.
    pub direct_symbol_count: usize,
    /// Files under this directory and every subdirectory, recursively.
    pub total_file_count: usize,
    /// Symbols under this directory and every subdirectory, recursively.
    pub total_symbol_count: usize,
    pub children: Vec<FileTreeNode>,
}

/// The hierarchical directory tree, rooted at `"."`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTree {
    pub root: FileTreeNode,
}

/// Baseline-aligned rule-based layer category (baseline tool schema doc
/// §7.2: `layers` -> `[{name, layer, reason}]`, `layer` in `{entry,
/// api, core, leaf, internal}` via "a rule-based classifier on
/// fan_in/fan_out + route/entry-point presence"). The baseline's exact
/// numeric thresholds are documented UNVERIFIED (no literal traced) --
/// this is a from-scratch, documented classifier matching the
/// *described* rule shape, not a byte-for-byte port of an unknown
/// constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerCategory {
    /// Has an entry-point (main.rs/lib.rs) or declares a route, and has
    /// callers from outside its own section (fan_in > 0 from another
    /// section) -- i.e. both an entry surface AND depended-upon.
    Entry,
    /// Declares at least one route -- the API/handler surface.
    Api,
    /// No entry-point/route of its own, but other sections depend on it
    /// (fan_in from other sections > 0) and it also depends on others
    /// (fan_out > 0) -- an internal, shared layer.
    Core,
    /// Depends on other sections (fan_out > 0) but nothing outside its
    /// own section depends on it back (fan_in == 0) -- a terminal
    /// consumer.
    Leaf,
    /// Neither depended-upon nor depending-on anything cross-section --
    /// an isolated/internal-only section.
    Internal,
}

/// One section's classification: [`CrateSection::name`], its
/// [`LayerCategory`], and a short human-readable reason string (the
/// baseline's `reason` field, per §7.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerClassification {
    pub name: String,
    pub layer: LayerCategory,
    pub reason: String,
}

/// The full, aspect-selected `get_architecture` report. Every field is
/// `None` unless its [`Aspect`] was requested (or [`Aspect::All`] was).
/// `PartialEq` only (not `Eq`): `cluster_cohesion` carries an `f64`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArchitectureReport {
    pub overview: Option<ArchitectureOverview>,
    pub structure: Option<Vec<CrateSection>>,
    pub dependencies: Option<Vec<DependencyEdge>>,
    pub routes: Option<Vec<RouteEntry>>,
    pub languages: Option<Vec<(String, usize)>>,
    pub packages: Option<Vec<PackageSection>>,
    pub entry_points: Option<Vec<EntryPoint>>,
    pub hotspots: Option<Vec<crate::analysis::HotspotScore>>,
    /// Baseline-aligned CALLS-fan-in-only hotspot ranking (baseline
    /// schema doc §7.3), populated alongside `hotspots` whenever
    /// [`Aspect::Hotspots`] is requested. `hotspots` above stays the
    /// original X06.3 total-degree metric for back-compat; this field
    /// is the parity-aligned one new callers should prefer for
    /// `get_architecture` behavioral parity.
    pub hotspot_entries: Option<Vec<HotspotEntry>>,
    pub boundaries: Option<Vec<Boundary>>,
    pub layers: Option<LayeringResult>,
    /// Baseline-aligned rule-based layer classification (baseline
    /// schema doc §7.2), populated alongside `layers` whenever
    /// [`Aspect::Layers`] is requested. `layers` above stays the
    /// topological/cycle-detecting ordering this pack's own hard tests
    /// require (the baseline's classifier has no cycle-detection
    /// concept at all); this field is the additional baseline-shaped
    /// view.
    pub layer_classification: Option<Vec<LayerClassification>>,
    pub file_tree: Option<FileTree>,
    pub clusters: Option<ClusteringResult>,
    /// Baseline-aligned per-cluster cohesion (baseline schema doc §7.4:
    /// `cohesion = internal_edges / (internal_edges + boundary_edges)`,
    /// ranked by descending member count), populated alongside
    /// `clusters` whenever [`Aspect::Clusters`] is requested. Kept
    /// separate from [`clustering::Cluster`] itself (rather than a new
    /// field there) so that struct's `Eq` derive is undisturbed by a
    /// floating-point field.
    pub cluster_cohesion: Option<Vec<ClusterCohesion>>,
}

/// One cluster's cohesion score (baseline schema doc §7.4): the
/// fraction of this cluster's total incident edges that stay inside the
/// cluster, `internal_edges / (internal_edges + boundary_edges)`. `1.0`
/// for a cluster with zero boundary edges (including singletons with no
/// edges at all, per this crate's "never silently drop data" posture --
/// the baseline drops clusters with `< CBM_CLUSTER_MIN_MEMBERS = 2`
/// members entirely, but `clustering::detect_clusters`'s existing hard
/// test explicitly requires singleton clusters to still appear, so
/// enforcer-memory's cohesion view reports them with a defined value
/// instead).
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterCohesion {
    pub cluster_id: String,
    pub member_count: usize,
    pub cohesion: f64,
}

/// Build a [`ArchitectureReport`] containing exactly the requested
/// `aspects` (expanding [`Aspect::All`] to every variant), optionally
/// scoped to files/symbols whose repo-relative path starts with
/// `path_prefix`. `hotspot_limit` bounds the `Hotspots`/`Overview`
/// sections the same way [`build_overview`] does; `max_iterations`
/// bounds [`clustering::detect_clusters`] for `Clusters`/`Layers`.
pub fn build_report(
    graph: &CodeGraph,
    aspects: &[Aspect],
    path_prefix: Option<&str>,
    hotspot_limit: usize,
    max_iterations: usize,
) -> ArchitectureReport {
    let wanted: BTreeSet<Aspect> = if aspects.contains(&Aspect::All) {
        [
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
        ]
        .into_iter()
        .collect()
    } else {
        aspects.iter().copied().collect()
    };

    let mut report = ArchitectureReport::default();

    // Clustering is shared by both `Clusters` and `Layers`, computed at
    // most once.
    let clustering_result =
        if wanted.contains(&Aspect::Clusters) || wanted.contains(&Aspect::Layers) {
            Some(filtered_clusters(graph, path_prefix, max_iterations))
        } else {
            None
        };

    if wanted.contains(&Aspect::Overview) {
        report.overview = Some(ArchitectureOverview {
            sections: crate_sections(graph, path_prefix),
            hotspots: CodeAdjacency::build(graph).hotspots(hotspot_limit),
            language_counts: language_counts(graph, path_prefix),
            total_files: graph
                .file_nodes()
                .filter(|f| path_matches(&f.rel_path, path_prefix))
                .count(),
            total_symbols: symbol_count_under(graph, path_prefix),
        });
    }
    if wanted.contains(&Aspect::Structure) {
        report.structure = Some(crate_sections(graph, path_prefix));
    }
    if wanted.contains(&Aspect::Dependencies) {
        report.dependencies = Some(dependency_edges(graph, path_prefix));
    }
    if wanted.contains(&Aspect::Routes) {
        // Baseline-aligned cap (baseline schema doc §7.2: `routes` ->
        // capped at 20). Applied only at this response-building site,
        // never inside `route_entries` itself, so other callers
        // (`layer_classification`'s route-presence detection,
        // `entry_points`'s RouteHandler detection) still see every
        // route regardless of the aspect-level display cap.
        let mut routes = route_entries(graph, path_prefix);
        routes.truncate(20);
        report.routes = Some(routes);
    }
    if wanted.contains(&Aspect::Languages) {
        report.languages = Some(language_counts(graph, path_prefix));
    }
    if wanted.contains(&Aspect::Packages) {
        report.packages = Some(package_sections(graph, path_prefix));
    }
    if wanted.contains(&Aspect::EntryPoints) {
        report.entry_points = Some(entry_points(graph, path_prefix));
    }
    if wanted.contains(&Aspect::Hotspots) {
        report.hotspots = Some(CodeAdjacency::build(graph).hotspots(hotspot_limit));
        report.hotspot_entries = Some(hotspot_entries(graph, path_prefix, hotspot_limit));
    }
    if wanted.contains(&Aspect::Boundaries) {
        report.boundaries = Some(boundaries(graph, path_prefix));
    }
    if wanted.contains(&Aspect::Layers) {
        if let Some(clusters) = &clustering_result {
            report.layers = Some(layering(clusters));
        }
        report.layer_classification = Some(layer_classification(graph, path_prefix));
    }
    if wanted.contains(&Aspect::FileTree) {
        report.file_tree = Some(file_tree(graph, path_prefix));
    }
    if wanted.contains(&Aspect::Clusters) {
        if let Some(clusters) = &clustering_result {
            report.cluster_cohesion = Some(cluster_cohesion(clusters));
        }
        report.clusters = clustering_result;
    }

    report
}

fn path_matches(rel_path: &str, prefix: Option<&str>) -> bool {
    match prefix {
        Some(p) if !p.is_empty() => rel_path.starts_with(p),
        _ => true,
    }
}

fn symbol_count_under(graph: &CodeGraph, prefix: Option<&str>) -> usize {
    graph
        .symbol_nodes()
        .filter(|s| {
            let rel_path = s.file_id.strip_prefix("file:").unwrap_or(&s.file_id);
            path_matches(rel_path, prefix)
        })
        .count()
}

/// Group a repo-relative path into a crate/section key: everything up
/// to (and including) the second path segment when the first segment
/// is `crates` (e.g. `crates/enforcer-memory/src/lib.rs` ->
/// `crates/enforcer-memory`), otherwise the first path segment, or
/// `"."` for a root-level file with no directory.
fn crate_map_key(rel_path: &str) -> String {
    let segments: Vec<&str> = rel_path.split('/').collect();
    match segments.as_slice() {
        [] => ".".to_string(),
        [_only] => ".".to_string(),
        ["crates", crate_name, ..] => format!("crates/{crate_name}"),
        [first, ..] => (*first).to_string(),
    }
}

fn crate_sections(graph: &CodeGraph, prefix: Option<&str>) -> Vec<CrateSection> {
    let mut sections: BTreeMap<String, CrateSection> = BTreeMap::new();

    for file in graph.file_nodes() {
        if !path_matches(&file.rel_path, prefix) {
            continue;
        }
        let crate_name = crate_map_key(&file.rel_path);
        let section = sections
            .entry(crate_name.clone())
            .or_insert_with(|| CrateSection {
                name: crate_name,
                file_count: 0,
                symbol_count: 0,
                rel_paths: Vec::new(),
            });
        section.file_count += 1;
        section.rel_paths.push(file.rel_path.clone());
    }

    for symbol in graph.symbol_nodes() {
        let rel_path = symbol
            .file_id
            .strip_prefix("file:")
            .unwrap_or(&symbol.file_id);
        if !path_matches(rel_path, prefix) {
            continue;
        }
        let crate_name = crate_map_key(rel_path);
        if let Some(section) = sections.get_mut(&crate_name) {
            section.symbol_count += 1;
        }
    }

    sections.into_values().collect()
}

fn language_counts(graph: &CodeGraph, prefix: Option<&str>) -> Vec<(String, usize)> {
    let mut language_counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in graph.file_nodes() {
        if !path_matches(&file.rel_path, prefix) {
            continue;
        }
        *language_counts
            .entry(format!("{:?}", file.language))
            .or_insert(0) += 1;
    }
    language_counts.into_iter().collect()
}

fn route_entries(graph: &CodeGraph, prefix: Option<&str>) -> Vec<RouteEntry> {
    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let mut entries: Vec<RouteEntry> = graph
        .routes()
        .iter()
        .filter_map(|r| {
            let declared_in = file_path_by_id.get(r.from_file_id.as_str()).copied();
            let declared_in = declared_in.unwrap_or(r.from_file_id.as_str());
            if !path_matches(declared_in, prefix) {
                return None;
            }
            Some(RouteEntry {
                method: r.method.clone(),
                path: r.path.clone(),
                declared_in: declared_in.to_string(),
                line: r.line,
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        a.declared_in
            .cmp(&b.declared_in)
            .then_with(|| a.line.cmp(&b.line))
    });
    entries
}

/// A file counts as a package manifest when its final path segment is
/// exactly `Cargo.toml` or `package.json` -- the two manifest formats
/// this crate's parity floor (D-06: Rust + TS/JS + Python + config)
/// actually indexes structurally.
fn is_manifest_file(rel_path: &str) -> bool {
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    name == "Cargo.toml" || name == "package.json"
}

fn dir_of(rel_path: &str) -> String {
    match rel_path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => ".".to_string(),
    }
}

fn package_sections(graph: &CodeGraph, prefix: Option<&str>) -> Vec<PackageSection> {
    let manifests: Vec<&str> = graph
        .file_nodes()
        .filter(|f| path_matches(&f.rel_path, prefix) && is_manifest_file(&f.rel_path))
        .map(|f| f.rel_path.as_str())
        .collect();

    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_names: Vec<(&str, &str)> = graph
        .symbol_nodes()
        .map(|s| (s.name.as_str(), s.id.as_str()))
        .collect();
    let symbol_file_by_id: BTreeMap<&str, &str> = graph
        .symbol_nodes()
        .map(|s| (s.id.as_str(), s.file_id.as_str()))
        .collect();

    let mut sections: Vec<PackageSection> = Vec::new();
    for manifest in manifests {
        let dir = dir_of(manifest);
        let is_under_dir = |rel_path: &str| -> bool {
            dir == "." || rel_path == dir || rel_path.starts_with(&format!("{dir}/"))
        };
        let members: Vec<String> = graph
            .file_nodes()
            .filter(|f| {
                path_matches(&f.rel_path, prefix)
                    && f.rel_path != manifest
                    && is_under_dir(&f.rel_path)
            })
            .map(|f| f.rel_path.clone())
            .collect();

        // Package-scoped, CALLS-only fan_in/fan_out (baseline-aligned
        // shape; see PackageSection docs for why these are computed
        // rather than left at the baseline's own always-zero stub).
        let (mut fan_in, mut fan_out) = (0usize, 0usize);
        for call in graph.calls() {
            let Some(&from_path) = file_path_by_id.get(call.from_file_id.as_str()) else {
                continue;
            };
            let from_inside = is_under_dir(from_path);
            let Some(to_symbol_id) = resolve_callee(&call.callee, &symbol_names) else {
                continue;
            };
            let Some(&to_file_id) = symbol_file_by_id.get(to_symbol_id) else {
                continue;
            };
            let Some(&to_path) = file_path_by_id.get(to_file_id) else {
                continue;
            };
            let to_inside = is_under_dir(to_path);
            match (from_inside, to_inside) {
                (true, false) => fan_out += 1,
                (false, true) => fan_in += 1,
                _ => {}
            }
        }

        sections.push(PackageSection {
            dir,
            manifest_rel_path: manifest.to_string(),
            member_file_count: members.len(),
            member_rel_paths: members,
            fan_in,
            fan_out,
        });
    }
    sections.sort_by(|a, b| a.manifest_rel_path.cmp(&b.manifest_rel_path));
    sections
}

fn entry_points(graph: &CodeGraph, prefix: Option<&str>) -> Vec<EntryPoint> {
    let mut entries: Vec<EntryPoint> = Vec::new();
    for file in graph.file_nodes() {
        if !path_matches(&file.rel_path, prefix) {
            continue;
        }
        let name = file.rel_path.rsplit('/').next().unwrap_or(&file.rel_path);
        if name == "main.rs" {
            entries.push(EntryPoint {
                rel_path: file.rel_path.clone(),
                kind: EntryPointKind::BinaryMain,
            });
        } else if name == "lib.rs" {
            entries.push(EntryPoint {
                rel_path: file.rel_path.clone(),
                kind: EntryPointKind::LibraryRoot,
            });
        }
    }
    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let mut route_files: BTreeSet<&str> = BTreeSet::new();
    for route in graph.routes() {
        if let Some(&rel_path) = file_path_by_id.get(route.from_file_id.as_str()) {
            if path_matches(rel_path, prefix) {
                route_files.insert(rel_path);
            }
        }
    }
    for rel_path in route_files {
        entries.push(EntryPoint {
            rel_path: rel_path.to_string(),
            kind: EntryPointKind::RouteHandler,
        });
    }
    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    entries
}

/// Resolve every import/call edge to a (from-section, to-section) pair
/// using the same best-effort suffix/name matching
/// [`crate::analysis::CodeAdjacency`] documents, and tally counts.
fn dependency_edges(graph: &CodeGraph, prefix: Option<&str>) -> Vec<DependencyEdge> {
    let file_paths: Vec<(&str, &str)> = graph
        .file_nodes()
        .filter(|f| path_matches(&f.rel_path, prefix))
        .map(|f| (f.rel_path.as_str(), f.id.as_str()))
        .collect();
    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_names: Vec<(&str, &str)> = graph
        .symbol_nodes()
        .map(|s| (s.name.as_str(), s.id.as_str()))
        .collect();
    let symbol_file_by_id: BTreeMap<&str, &str> = graph
        .symbol_nodes()
        .map(|s| (s.id.as_str(), s.file_id.as_str()))
        .collect();

    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();

    for import in graph.imports() {
        let Some(&from_path) = file_path_by_id.get(import.from_file_id.as_str()) else {
            continue;
        };
        if !path_matches(from_path, prefix) {
            continue;
        }
        if let Some(to_path) = resolve_module_path(&import.module_path, &file_paths) {
            let from_section = crate_map_key(from_path);
            let to_section = crate_map_key(to_path);
            if from_section != to_section {
                *counts.entry((from_section, to_section)).or_insert(0) += 1;
            }
        }
    }

    for call in graph.calls() {
        let Some(&from_path) = file_path_by_id.get(call.from_file_id.as_str()) else {
            continue;
        };
        if !path_matches(from_path, prefix) {
            continue;
        }
        if let Some(to_symbol_id) = resolve_callee(&call.callee, &symbol_names) {
            if let Some(&to_file_id) = symbol_file_by_id.get(to_symbol_id) {
                if let Some(&to_path) = file_path_by_id.get(to_file_id) {
                    if path_matches(to_path, prefix) {
                        let from_section = crate_map_key(from_path);
                        let to_section = crate_map_key(to_path);
                        if from_section != to_section {
                            *counts.entry((from_section, to_section)).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    counts
        .into_iter()
        .map(|((from, to), count)| DependencyEdge { from, to, count })
        .collect()
}

fn resolve_module_path<'a>(
    module_path: &str,
    file_paths: &[(&'a str, &'a str)],
) -> Option<&'a str> {
    let needle = module_path
        .trim_start_matches("./")
        .trim_start_matches("../");
    let last_segment = needle.rsplit(['/', ':', '.']).next().unwrap_or(needle);
    if last_segment.is_empty() {
        return None;
    }
    file_paths
        .iter()
        .find(|(rel_path, _)| {
            let stem = rel_path.rsplit('/').next().unwrap_or(rel_path);
            let stem = stem.split('.').next().unwrap_or(stem);
            stem == last_segment || rel_path.ends_with(last_segment)
        })
        .map(|(rel_path, _)| *rel_path)
}

fn resolve_callee<'a>(callee: &str, symbol_names: &[(&'a str, &'a str)]) -> Option<&'a str> {
    let last_segment = callee.rsplit(['.', ':']).next().unwrap_or(callee);
    symbol_names
        .iter()
        .find(|(name, _)| *name == callee || *name == last_segment)
        .map(|(_, id)| *id)
}

/// Baseline-aligned `boundaries` aspect (baseline tool schema doc §7.2:
/// `[{from, to, call_count}]`, "cross-package CALLS edge counts") --
/// directed, CALLS-edges only (never imports), matching the baseline's
/// semantics rather than [`dependency_edges`]'s broader import+call
/// mix used by [`Aspect::Dependencies`].
fn boundaries(graph: &CodeGraph, prefix: Option<&str>) -> Vec<Boundary> {
    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_names: Vec<(&str, &str)> = graph
        .symbol_nodes()
        .map(|s| (s.name.as_str(), s.id.as_str()))
        .collect();
    let symbol_file_by_id: BTreeMap<&str, &str> = graph
        .symbol_nodes()
        .map(|s| (s.id.as_str(), s.file_id.as_str()))
        .collect();

    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for call in graph.calls() {
        let Some(&from_path) = file_path_by_id.get(call.from_file_id.as_str()) else {
            continue;
        };
        if !path_matches(from_path, prefix) {
            continue;
        }
        let Some(to_symbol_id) = resolve_callee(&call.callee, &symbol_names) else {
            continue;
        };
        let Some(&to_file_id) = symbol_file_by_id.get(to_symbol_id) else {
            continue;
        };
        let Some(&to_path) = file_path_by_id.get(to_file_id) else {
            continue;
        };
        if !path_matches(to_path, prefix) {
            continue;
        }
        let from_section = crate_map_key(from_path);
        let to_section = crate_map_key(to_path);
        if from_section != to_section {
            *counts.entry((from_section, to_section)).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .map(|((from, to), call_count)| Boundary {
            from,
            to,
            call_count,
        })
        .collect()
}

/// Compute [`clustering::detect_clusters`] over a graph view restricted
/// to `prefix` -- clustering itself has no path-filter parameter, so
/// scoping happens by rebuilding a filtered [`CodeGraph`] would be
/// wasteful; instead the unfiltered clustering result's clusters are
/// pruned to members whose path matches `prefix`, and any cluster left
/// with no members after pruning is dropped. Inter-cluster edges
/// referencing a dropped cluster are dropped too.
fn filtered_clusters(
    graph: &CodeGraph,
    prefix: Option<&str>,
    max_iterations: usize,
) -> ClusteringResult {
    let result = clustering::detect_clusters(graph, max_iterations);
    if prefix.is_none() {
        return result;
    }

    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_file_by_id: BTreeMap<&str, &str> = graph
        .symbol_nodes()
        .map(|s| (s.id.as_str(), s.file_id.as_str()))
        .collect();

    let node_matches = |node_id: &str| -> bool {
        let rel_path = file_path_by_id.get(node_id).copied().or_else(|| {
            symbol_file_by_id
                .get(node_id)
                .and_then(|file_id| file_path_by_id.get(*file_id).copied())
        });
        match rel_path {
            Some(p) => path_matches(p, prefix),
            None => false,
        }
    };

    let mut kept_cluster_ids: BTreeSet<String> = BTreeSet::new();
    let clusters: Vec<clustering::Cluster> = result
        .clusters
        .into_iter()
        .filter_map(|mut cluster| {
            cluster.member_node_ids.retain(|id| node_matches(id));
            cluster.file_ids.retain(|id| node_matches(id));
            cluster.symbol_ids.retain(|id| node_matches(id));
            if cluster.member_node_ids.is_empty() {
                None
            } else {
                kept_cluster_ids.insert(cluster.id.clone());
                Some(cluster)
            }
        })
        .collect();

    let inter_cluster_edges = result
        .inter_cluster_edges
        .into_iter()
        .filter(|e| {
            kept_cluster_ids.contains(&e.from_cluster) && kept_cluster_ids.contains(&e.to_cluster)
        })
        .collect();

    ClusteringResult {
        clusters,
        inter_cluster_edges,
    }
}

/// Baseline-aligned per-cluster cohesion (baseline schema doc §7.4:
/// `cohesion = internal_edges / (internal_edges + boundary_edges)`).
/// "Internal edges" is approximated here as each cluster's member
/// count minus one summed appropriately -- more precisely, since
/// [`ClusteringResult`] does not retain a raw internal-edge count
/// per cluster (only the cross-cluster [`InterClusterEdge`] list),
/// internal edge count is recovered as `member_count - 1` treated as a
/// lower-bound proxy for a connected cluster (label propagation only
/// merges nodes that were already linked, so every cluster with `n`
/// members has at least `n - 1` internal edges forming a spanning
/// structure); boundary edges are the exact sum of every
/// [`InterClusterEdge`] touching this cluster (either direction).
/// Ranked by descending member count (baseline: "Clusters ranked by
/// descending member count").
fn cluster_cohesion(result: &ClusteringResult) -> Vec<ClusterCohesion> {
    let mut boundary_edges: BTreeMap<&str, usize> = BTreeMap::new();
    for edge in &result.inter_cluster_edges {
        *boundary_edges
            .entry(edge.from_cluster.as_str())
            .or_insert(0) += edge.count;
        *boundary_edges.entry(edge.to_cluster.as_str()).or_insert(0) += edge.count;
    }

    let mut entries: Vec<ClusterCohesion> = result
        .clusters
        .iter()
        .map(|cluster| {
            let member_count = cluster.size();
            let internal_edges = member_count.saturating_sub(1);
            let boundary = boundary_edges
                .get(cluster.id.as_str())
                .copied()
                .unwrap_or(0);
            let denom = internal_edges + boundary;
            let cohesion = if denom == 0 {
                1.0
            } else {
                internal_edges as f64 / denom as f64
            };
            ClusterCohesion {
                cluster_id: cluster.id.clone(),
                member_count,
                cohesion,
            }
        })
        .collect();

    entries.sort_by(|a, b| {
        b.member_count
            .cmp(&a.member_count)
            .then_with(|| a.cluster_id.cmp(&b.cluster_id))
    });
    entries
}

/// Baseline-aligned rule-based `layers` classification (baseline schema
/// doc §7.2: `[{name, layer, reason}]`, categories `entry|api|core|
/// leaf|internal` "via a rule-based classifier on fan_in/fan_out +
/// route/entry-point presence"). Operates per [`CrateSection`] (see
/// [`crate_sections`]), using [`boundaries`] (CALLS-only, directed) for
/// fan_in/fan_out counts and [`entry_points`]/[`route_entries`] for
/// entry-surface detection. Deterministic: sections are visited in
/// [`crate_sections`]' `BTreeMap`-derived order and every count is an
/// exact sum, no sampling.
fn layer_classification(graph: &CodeGraph, prefix: Option<&str>) -> Vec<LayerClassification> {
    let sections = crate_sections(graph, prefix);
    let cross_edges = boundaries(graph, prefix);
    let entries = entry_points(graph, prefix);
    let routes = route_entries(graph, prefix);

    let mut fan_in: BTreeMap<&str, usize> = BTreeMap::new();
    let mut fan_out: BTreeMap<&str, usize> = BTreeMap::new();
    for edge in &cross_edges {
        *fan_out.entry(edge.from.as_str()).or_insert(0) += edge.call_count;
        *fan_in.entry(edge.to.as_str()).or_insert(0) += edge.call_count;
    }

    let mut has_entry_point: BTreeSet<String> = BTreeSet::new();
    for entry in &entries {
        has_entry_point.insert(crate_map_key(&entry.rel_path));
    }
    let mut has_route: BTreeSet<String> = BTreeSet::new();
    for route in &routes {
        has_route.insert(crate_map_key(&route.declared_in));
    }

    sections
        .iter()
        .map(|section| {
            let name = section.name.clone();
            let this_fan_in = fan_in.get(name.as_str()).copied().unwrap_or(0);
            let this_fan_out = fan_out.get(name.as_str()).copied().unwrap_or(0);
            let is_entry = has_entry_point.contains(&name);
            let is_api = has_route.contains(&name);

            let (layer, reason) = if is_api {
                (
                    LayerCategory::Api,
                    "declares at least one route".to_string(),
                )
            } else if is_entry && this_fan_in > 0 {
                (
                    LayerCategory::Entry,
                    format!("has a binary/library entry point and {this_fan_in} incoming cross-section call(s)"),
                )
            } else if this_fan_in > 0 && this_fan_out > 0 {
                (
                    LayerCategory::Core,
                    format!(
                        "depended upon by other sections ({this_fan_in} incoming) and depends on others ({this_fan_out} outgoing)"
                    ),
                )
            } else if this_fan_out > 0 {
                (
                    LayerCategory::Leaf,
                    format!("depends on other sections ({this_fan_out} outgoing) but nothing depends on it back"),
                )
            } else {
                (
                    LayerCategory::Internal,
                    "no cross-section calls in either direction".to_string(),
                )
            };

            LayerClassification { name, layer, reason }
        })
        .collect()
}

/// Topologically order [`ClusteringResult::clusters`] by dependency
/// direction (Kahn's algorithm over the inter-cluster edges as a
/// dependency DAG: `from -> to` means `from` depends on `to`, so `to`
/// must appear in an earlier-or-equal layer). Deterministic: every
/// layer's cluster ids are sorted, and ties in Kahn's queue are broken
/// by cluster id order, never insertion/hash order. A cluster
/// participating in a cycle (in-degree never reaches zero) is reported
/// in [`LayeringResult::cycle_cluster_ids`] instead of panicking or
/// being silently dropped.
fn layering(clusters: &ClusteringResult) -> LayeringResult {
    let all_ids: BTreeSet<String> = clusters.clusters.iter().map(|c| c.id.clone()).collect();
    if all_ids.is_empty() {
        return LayeringResult::default();
    }

    // dependency direction: `to` is depended upon by `from`, so `to`
    // must be placed before (or in the same layer as, if no direct
    // edge) `from`. We compute layers from the "most depended upon"
    // end outward: a node with in-degree 0 in the "depends on" graph
    // (i.e. nothing depends on it going the other way) ... To keep this
    // simple and correct, treat inter_cluster_edges as `from` depends
    // on `to`; layer 0 = clusters nothing depends on that themselves
    // have no outstanding dependencies once earlier layers are removed
    // -- i.e. process the graph with edges reversed (`to` -> `from`)
    // and peel zero-in-degree nodes of the ORIGINAL `from->to` sense,
    // meaning: a cluster can be placed once every cluster it depends on
    // (every `to` for its `from` edges) has already been placed.
    let mut remaining_deps: BTreeMap<String, BTreeSet<String>> = all_ids
        .iter()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();
    for edge in &clusters.inter_cluster_edges {
        remaining_deps
            .entry(edge.from_cluster.clone())
            .or_default()
            .insert(edge.to_cluster.clone());
    }

    let mut placed: BTreeSet<String> = BTreeSet::new();
    let mut layers: Vec<Layer> = Vec::new();
    let mut layer_index = 0usize;

    loop {
        let mut ready: Vec<String> = remaining_deps
            .iter()
            .filter(|(id, deps)| {
                !placed.contains(*id)
                    && deps
                        .iter()
                        .all(|d| placed.contains(d) || !all_ids.contains(d))
            })
            .map(|(id, _)| id.clone())
            .collect();
        ready.sort();

        if ready.is_empty() {
            break;
        }

        for id in &ready {
            placed.insert(id.clone());
        }
        layers.push(Layer {
            index: layer_index,
            cluster_ids: ready,
        });
        layer_index += 1;
    }

    let cycle_cluster_ids: Vec<String> = all_ids
        .into_iter()
        .filter(|id| !placed.contains(id))
        .collect();

    LayeringResult {
        layers,
        cycle_cluster_ids,
    }
}

fn file_tree(graph: &CodeGraph, prefix: Option<&str>) -> FileTree {
    // dir path -> (direct_file_count, direct_symbol_count).
    let mut direct_files: BTreeMap<String, usize> = BTreeMap::new();
    let mut direct_symbols: BTreeMap<String, usize> = BTreeMap::new();
    let mut all_dirs: BTreeSet<String> = BTreeSet::new();
    all_dirs.insert(".".to_string());

    let file_dir_by_id: BTreeMap<&str, String> = graph
        .file_nodes()
        .filter(|f| path_matches(&f.rel_path, prefix))
        .map(|f| (f.id.as_str(), dir_of(&f.rel_path)))
        .collect();

    for file in graph.file_nodes() {
        if !path_matches(&file.rel_path, prefix) {
            continue;
        }
        let dir = dir_of(&file.rel_path);
        *direct_files.entry(dir.clone()).or_insert(0) += 1;
        register_ancestors(&mut all_dirs, &dir);
    }

    for symbol in graph.symbol_nodes() {
        if let Some(dir) = file_dir_by_id.get(symbol.file_id.as_str()) {
            *direct_symbols.entry(dir.clone()).or_insert(0) += 1;
        }
    }

    let root = build_tree_node(".", &all_dirs, &direct_files, &direct_symbols);
    FileTree { root }
}

fn register_ancestors(all_dirs: &mut BTreeSet<String>, dir: &str) {
    all_dirs.insert(dir.to_string());
    let mut current = dir;
    while let Some((parent, _)) = current.rsplit_once('/') {
        all_dirs.insert(parent.to_string());
        current = parent;
    }
}

fn direct_children<'a>(all_dirs: &'a BTreeSet<String>, dir: &str) -> Vec<&'a str> {
    all_dirs
        .iter()
        .filter(|candidate| {
            if *candidate == dir {
                return false;
            }
            // A direct child's own `dir_of(...)` is exactly `dir` (both
            // for `dir == "."`, where a top-level candidate like
            // `"crates"` has no `/` and `dir_of` returns `"."`, and for
            // any nested `dir`).
            dir_of(candidate) == dir
        })
        .map(String::as_str)
        .collect()
}

fn build_tree_node(
    dir: &str,
    all_dirs: &BTreeSet<String>,
    direct_files: &BTreeMap<String, usize>,
    direct_symbols: &BTreeMap<String, usize>,
) -> FileTreeNode {
    let direct_file_count = direct_files.get(dir).copied().unwrap_or(0);
    let direct_symbol_count = direct_symbols.get(dir).copied().unwrap_or(0);

    let children: Vec<FileTreeNode> = direct_children(all_dirs, dir)
        .into_iter()
        .map(|child_dir| build_tree_node(child_dir, all_dirs, direct_files, direct_symbols))
        .collect();

    let total_file_count =
        direct_file_count + children.iter().map(|c| c.total_file_count).sum::<usize>();
    let total_symbol_count =
        direct_symbol_count + children.iter().map(|c| c.total_symbol_count).sum::<usize>();

    FileTreeNode {
        dir: dir.to_string(),
        direct_file_count,
        direct_symbol_count,
        total_file_count,
        total_symbol_count,
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::{CodeGraph, Manifest};
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
        let names: Vec<&str> = overview.sections.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"crates/foo"));
        assert!(names.contains(&"crates/bar"));
        assert_eq!(overview.total_files, 2);
        assert_eq!(overview.total_symbols, 2);
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
        assert!(overview
            .language_counts
            .iter()
            .any(|(lang, count)| lang == "Rust" && *count == 2));
        assert!(
            !overview.hotspots.is_empty(),
            "expected at least one hotspot entry"
        );
        Ok(())
    }

    #[test]
    fn empty_graph_produces_empty_overview_not_panic() {
        let graph = CodeGraph::new();
        let overview = build_overview(&graph, 5);
        assert!(overview.sections.is_empty());
        assert_eq!(overview.total_files, 0);
        assert_eq!(overview.total_symbols, 0);
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

    // --- hard test: entry_points aspect ----------------------------------

    #[test]
    fn entry_points_aspect_finds_main_and_lib_and_route_handler() -> TestResult {
        let dir = tempfile::tempdir()?;
        let graph = build_two_crate_fixture(dir.path())?;

        let report = build_report(&graph, &[Aspect::EntryPoints], None, 10, 20);
        let entries = report.entry_points.ok_or("expected entry_points section")?;
        assert!(entries.iter().any(
            |e| e.rel_path == "crates/api/src/main.rs" && e.kind == EntryPointKind::BinaryMain
        ));
        assert!(entries.iter().any(
            |e| e.rel_path == "crates/core/src/lib.rs" && e.kind == EntryPointKind::LibraryRoot
        ));
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
        assert!(
            deps.iter()
                .any(|d| d.from == "crates/api" && d.to == "crates/core"),
            "expected crates/api -> crates/core dependency edge, got {deps:?}"
        );
        assert!(
            !deps
                .iter()
                .any(|d| d.from == "crates/core" && d.to == "crates/api"),
            "core must not depend on api"
        );
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
        assert!(boundaries
            .iter()
            .any(|b| b.from == "crates/api" && b.to == "crates/core" && b.call_count > 0));
        assert!(!boundaries
            .iter()
            .any(|b| b.from == "crates/core" && b.to == "crates/api"));
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
            let core_layer =
                layer_of(&core_cluster.id).ok_or("core cluster missing from layers")?;
            let api_layer = layer_of(&api_cluster.id).ok_or("api cluster missing from layers")?;
            assert!(
                core_layer <= api_layer,
                "expected core's layer ({core_layer}) <= api's layer ({api_layer}) since api depends on core"
            );
        }
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

        let result = layering(&clusters);
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
}
