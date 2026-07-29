//! X06.3 + X06.P4: architecture overview / repo mind map.
//!
//! Answers the "architecture overview" and "repo mind map" hard
//! requirements, extended (X06.P4, the wave-3 parity push) to the full
//! `get_architecture` aspect surface. Aspect naming/count and every
//! per-aspect response shape below are aligned to
//! `docs/plans/enforcer-selfhost-plan/refs/x06-baseline-tool-schemas.md`
//! Â§7 (the ground-truth C-source extraction that landed after this
//! module's first pass and superseded the higher-level scout digest's
//! summary) -- **semantics, not SQL**: enforcer-memory re-derives every
//! metric from its own graph model rather than porting the baseline's C
//! queries, per MEMORY_RETRIEVAL_BORROW_POLICY (behavioral parity only,
//! never code, for the C baseline).
//!
//! - [`Aspect::All`] -- every aspect below, populated together (baseline
//!   Â§7.2: a meta-token, not its own response key);
//! - [`Aspect::Overview`] -- crate/module map + hotspots + language
//!   composition (the original X06.3 slice, unchanged shape; baseline
//!   Â§7.2: "all aspects except file_tree" -- a meta-token there, but
//!   this crate keeps it as its own typed section since Rust callers
//!   need a concrete return type regardless);
//! - [`Aspect::Structure`] -- the crate/module sections alone;
//! - [`Aspect::Dependencies`] -- directed crate-section -> crate-section
//!   edges, derived from resolved import/call edges (broader than
//!   [`Aspect::Boundaries`], which is CALLS-only per baseline Â§7.2);
//! - [`Aspect::Routes`] -- every declared HTTP-style route, capped at 20
//!   (baseline Â§7.2: `routes` capped at 20);
//! - [`Aspect::Languages`] -- language composition counts;
//! - [`Aspect::Packages`] -- detected package manifests (`Cargo.toml`,
//!   `package.json`), the files under each, and CALLS-only
//!   `fan_in`/`fan_out` (baseline Â§7.2 documents its own `fan_in`/
//!   `fan_out` as "always 0... UNVERIFIED why" -- a likely partial placeholder this
//!   crate does not reproduce; real counts are computed instead);
//! - [`Aspect::EntryPoints`] -- binary/library entry files (`main.rs`,
//!   `lib.rs`) plus route-declaring files;
//! - [`Aspect::Hotspots`] -- BOTH the original X06.3 total-degree metric
//!   ([`ArchitectureOverview::hotspots`]/[`crate::analysis::HotspotScore`],
//!   kept for back-compat) AND a baseline-aligned CALLS-fan-in-only
//!   ranking over Function/Test symbols with test files excluded
//!   ([`HotspotEntry`], baseline Â§7.3's exact SQL semantics re-derived);
//! - [`Aspect::Boundaries`] -- directed, CALLS-only cross-section edge
//!   counts, `{from, to, call_count}` (baseline Â§7.2's exact shape --
//!   this module's first pass had this aspect wrong as an undirected,
//!   import+call-mixed pair; corrected against the landed baseline
//!   schema doc);
//! - [`Aspect::Layers`] -- BOTH a topological ordering of
//!   [`crate::analysis::clustering`] communities by dependency
//!   direction with cycles reported rather than panicking (this pack's
//!   own hard-test requirement; the baseline classifier has no
//!   cycle-detection concept at all) AND a baseline-aligned rule-based
//!   `{name, layer, reason}` classification into `entry|api|core|leaf|
//!   internal` categories ([`LayerClassification`], baseline Â§7.2);
//! - [`Aspect::FileTree`] -- a hierarchical directory tree with
//!   per-directory file/symbol counts (baseline Â§7.2 returns a *flat*
//!   `[{path,type,children}]` array instead; this module keeps the
//!   richer nested shape this pack's own hard test explicitly requires
//!   -- OWNER_INTENT's "equal or better quality" clause, not a gap);
//! - [`Aspect::Clusters`] -- the de-facto module communities themselves
//!   ([`crate::analysis::clustering::detect_clusters`], deterministic
//!   label propagation, never Leiden's randomized tie-breaks) plus
//!   baseline-aligned per-cluster `cohesion`
//!   ([`ClusterCohesion`], baseline Â§7.4's exact formula). Baseline
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
//! contribute to any requested section (baseline Â§7.1: `path` is a
//! "directory-prefix scope, applied uniformly across every requested
//! aspect").

use crate::analysis::clustering::{self, ClusteringResult};
use crate::analysis::CodeAdjacency;
use crate::code_graph::CodeGraph;
use crate::owned_boundary::{Retained, RetainedDisplay};
use enforcer_domain::memory_types::{
    ArchitectureClusterId, ArchitectureCohesion, ArchitectureHotspotLimit, ArchitectureItemCount,
    ArchitectureLanguage, ArchitectureLayerIndex, ArchitectureMaxIterations, ArchitectureName,
    ArchitectureNodeId, ArchitecturePath, ArchitecturePathMatch, ArchitectureReason,
    ArchitectureReportPath, ArchitectureRouteMethod, Aspect, EntryPointKind, LayerCategory,
    ParsedCallee, ParsedModulePath, ParsedSymbolName,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

macro_rules! scope_matches {
    ($scope:expr, $path:expr) => {
        // BRAND-INVARIANT: the macro only unwraps the canonical
        // ArchitecturePathMatch returned by ArchitectureScope::includes.
        bool::from($scope.includes($path))
    };
}

/// An owned, repository-relative directory inside the architecture domain.
///
/// BRAND-INVARIANT: this value is either the repository root (`"."`) or the
/// directory portion of an [`ArchitecturePath`]. It is never an absolute path,
/// never has a trailing separator, and is used only as the typed key for
/// architecture file-tree aggregation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ArchitectureDirectory(String);

impl ArchitectureDirectory {
    fn root() -> Self {
        // ALLOC-JUSTIFICATION: the root is retained as an owned directory key
        // beside all graph-derived aggregation keys.
        Self(".".retained())
    }

    fn from_file_path(path: ArchitecturePath<'_>) -> Self {
        match path.as_str().rsplit_once('/') {
            // ALLOC-JUSTIFICATION: graph paths are borrowed input while the
            // directory key must outlive each aggregation pass.
            Some((dir, _)) => Self(dir.retained()),
            None => Self::root(),
        }
    }

    fn parent(&self) -> Option<Self> {
        self.0.rsplit_once('/').map(|(parent, _)| {
            // ALLOC-JUSTIFICATION: each ancestor is retained as a distinct
            // owned key in the report's directory hierarchy.
            Self(parent.retained())
        })
    }

    fn contains_path(&self, path: ArchitecturePath<'_>) -> ArchitecturePathMatch {
        (self.0 == "."
            || path.as_str() == self.0
            || path
                .as_str()
                .strip_prefix(&self.0)
                .is_some_and(|rest| rest.starts_with('/')))
        .into()
    }
}

/// An owned crate or top-level section identifier used during architecture
/// aggregation.
///
/// BRAND-INVARIANT: the value is derived only from a repository-relative
/// [`ArchitecturePath`], using the crate-map grouping documented by
/// [`CrateSection`]. It is an internal map key, not an unvalidated caller
/// supplied identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ArchitectureSectionGroup(String);

impl ArchitectureSectionGroup {
    fn from_path(path: ArchitecturePath<'_>) -> Self {
        let mut segments = path.as_str().split('/');
        let first = segments.next();
        let second = segments.next();
        match (first, second) {
            // ALLOC-JUSTIFICATION: section keys own their normalized grouping
            // independently of the graph's borrowed file path.
            (Some("crates"), Some(crate_name)) => Self(format!("crates/{crate_name}")),
            (Some(first), Some(_)) => Self(first.retained()),
            _ => Self(".".retained()),
        }
    }
}

/// The optional repository-relative prefix applied uniformly to one report.
///
/// BRAND-INVARIANT: when present, the prefix is compared only against the
/// slash-normalized paths held by [`ArchitecturePath`]. Absolute and parent
/// traversal prefixes are rejected by matching no paths, so malformed caller
/// input can never widen an architecture query to the whole repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArchitectureScope<'a> {
    prefix: Option<ArchitecturePath<'a>>,
}

impl ArchitectureScope<'_> {
    fn includes(self, path: ArchitecturePath<'_>) -> ArchitecturePathMatch {
        self.prefix
            .is_none_or(|prefix| {
                if prefix.as_str().starts_with('/')
                    || prefix.as_str().split('/').any(|segment| segment == "..")
                {
                    return false;
                }
                let normalized_prefix = prefix.as_str().trim_end_matches('/');
                normalized_prefix.is_empty()
                    || path.as_str() == normalized_prefix
                    || path
                        .as_str()
                        .strip_prefix(normalized_prefix)
                        .is_some_and(|remainder| remainder.starts_with('/'))
            })
            .into()
    }
}

/// One crate/top-level-directory section of the architecture overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateSection {
    /// The top-level path segment (e.g. `crates/enforcer-memory` or the
    /// crate name if the repo root itself contains `Cargo.toml` files
    /// one level down) this section groups.
    pub name: ArchitectureName,
    pub file_count: ArchitectureItemCount,
    pub symbol_count: ArchitectureItemCount,
    pub rel_paths: Vec<ArchitectureReportPath>,
}

/// The full architecture overview: crate map + hotspots + language
/// composition. Constructed by [`build_overview`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchitectureOverview {
    sections: ArchitectureSections,
    hotspots: ArchitectureHotspots,
    language_counts: ArchitectureLanguageCounts,
    total_files: ArchitectureFileCount,
    total_symbols: ArchitectureSymbolCount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArchitectureSections(Vec<CrateSection>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArchitectureHotspots(Vec<crate::analysis::HotspotScore>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArchitectureLanguageCounts(Vec<(ArchitectureLanguage, ArchitectureItemCount)>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArchitectureFileCount(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArchitectureSymbolCount(usize);

impl ArchitectureOverview {
    /// Crate/top-level-directory sections in deterministic report order.
    pub fn sections(&self) -> &[CrateSection] {
        &self.sections.0
    }

    /// Highest-degree symbols retained for this overview's requested limit.
    pub fn hotspots(&self) -> &[crate::analysis::HotspotScore] {
        &self.hotspots.0
    }

    /// Per-language file counts in the MCP's stable JSON representation.
    pub fn language_counts_json(&self) -> Value {
        json!(self.language_counts.0)
    }

    /// Indexed-file count in the MCP's stable JSON representation.
    pub fn total_files_json(&self) -> Value {
        json!(self.total_files.0)
    }

    /// Indexed-symbol count in the MCP's stable JSON representation.
    pub fn total_symbols_json(&self) -> Value {
        json!(self.total_symbols.0)
    }
}

/// Build the architecture overview for `graph`. `hotspot_limit` bounds
/// how many top hotspot entries are retained (the workpack does not
/// mandate a specific number; callers pick per their MCP/CLI surface).
pub fn build_overview(
    graph: &CodeGraph,
    hotspot_limit: impl Into<ArchitectureHotspotLimit>,
) -> ArchitectureOverview {
    let hotspot_limit = hotspot_limit.into();
    let scope = ArchitectureScope { prefix: None };
    let sections = crate_sections(graph, scope);
    let language_counts = language_counts(graph, scope);
    let adjacency = CodeAdjacency::build(graph);
    let hotspots = adjacency.hotspots(hotspot_limit.get());
    let total_files = graph
        .file_nodes()
        .filter(|f| scope_matches!(scope, ArchitecturePath::from(&f.rel_path)))
        .count();
    let total_symbols = symbol_count_under(graph, scope);

    ArchitectureOverview {
        sections: ArchitectureSections(sections),
        hotspots: ArchitectureHotspots(hotspots),
        language_counts: ArchitectureLanguageCounts(language_counts),
        total_files: ArchitectureFileCount(total_files),
        total_symbols: ArchitectureSymbolCount(total_symbols.into()),
    }
}

// ---------------------------------------------------------------------
// X06.P4: full aspect surface
// ---------------------------------------------------------------------

/// Which `get_architecture` aspect(s) a caller wants. `All` expands to
/// every other variant at request time ([`build_report`]).
/// One route entry (a re-shaping of [`crate::code_graph::RouteEdge`]
/// with the declaring file's path attached, since the raw edge only
/// carries the file *id*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntry {
    pub method: ArchitectureRouteMethod,
    pub path: ArchitectureReportPath,
    pub declared_in: ArchitectureReportPath,
    pub line: enforcer_domain::memory_types::GraphSourceLine,
}

/// One detected package manifest (`Cargo.toml`/`package.json`) and the
/// files that live under its directory. `fan_in`/`fan_out` are
/// baseline-aligned (baseline schema doc Â§7.2: `packages` ->
/// `[{name, node_count, fan_in, fan_out}]`) -- CALLS-only,
/// cross-package edge counts, computed here rather than left at zero
/// (the baseline doc flags its own `fan_in`/`fan_out` as "always 0...
/// UNVERIFIED why," i.e. likely a partial implementation gap, not a documented design
/// ceiling worth reproducing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSection {
    /// The manifest's own containing directory, `"."` for a root-level
    /// manifest.
    pub dir: ArchitectureReportPath,
    pub manifest_rel_path: ArchitectureReportPath,
    pub member_file_count: ArchitectureItemCount,
    pub member_rel_paths: Vec<ArchitectureReportPath>,
    /// Incoming CALLS edges from outside this package's own directory.
    pub fan_in: ArchitectureItemCount,
    /// Outgoing CALLS edges to outside this package's own directory.
    pub fan_out: ArchitectureItemCount,
}

/// One entry-point file (`main.rs`, `lib.rs`, or any file declaring at
/// least one route -- the two shapes of "where execution begins" this
/// crate's graph can currently see).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPoint {
    pub rel_path: ArchitectureReportPath,
    pub kind: EntryPointKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ArchitectureSectionId(String);

impl ArchitectureSectionId {
    fn from_key(key: ArchitectureSectionGroup) -> Self {
        Self(key.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CrossSectionEdgeCount(usize);

/// A directed dependency edge between two crate/module sections (see
/// [`CrateSection::name`]), with the number of resolved import/call
/// edges crossing from `from` into `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    from: ArchitectureSectionId,
    to: ArchitectureSectionId,
    count: CrossSectionEdgeCount,
}

/// A directed cross-section CALLS-edge count: `from` section calls into
/// `to` section `call_count` times. Baseline-aligned shape (baseline
/// tool schema doc Â§7.2: `boundaries` -> `[{from, to, call_count}]`,
/// "cross-package CALLS edge counts") -- directed, CALLS-only, distinct
/// from [`Aspect::Dependencies`]'s broader import+call edge set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary {
    from: ArchitectureSectionId,
    to: ArchitectureSectionId,
    call_count: CrossSectionEdgeCount,
}

/// One layer in the dependency-direction topological ordering of
/// [`crate::analysis::clustering`] communities: every cluster id in
/// this layer depends only on clusters in strictly earlier layers (or
/// on nothing). Layer 0 is the most upstream (depended-upon-by-everyone)
/// end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub index: ArchitectureLayerIndex,
    pub cluster_ids: Vec<ArchitectureClusterId>,
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
    pub cycle_cluster_ids: Vec<ArchitectureClusterId>,
}

/// One baseline-aligned hotspot entry (baseline tool schema doc Â§7.3:
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
    pub name: ArchitectureName,
    /// This crate's node id (`sym:<rel_path>:<line>:<name>`), the
    /// closest equivalent to baseline's `qualified_name`.
    pub node_id: ArchitectureNodeId,
    pub fan_in: ArchitectureItemCount,
}

/// A file counts as a "test file" for hotspot exclusion when its path
/// contains a `test`/`tests`/`spec` segment or a `_test`/`.test`
/// filename marker -- the same class of heuristic the baseline schema
/// doc documents for its own hotspot SQL (`file_path NOT LIKE
/// '%test%'`) and for `trace_path`'s `include_tests` filter (Â§5.4),
/// re-expressed here rather than copied (BORROW_POLICY: behavior spec
/// only, no C code exists to copy).
fn looks_like_test_path(rel_path: ArchitecturePath<'_>) -> ArchitecturePathMatch {
    let lower = rel_path.as_str().to_lowercase();
    (lower.contains("test") || lower.contains("/spec/") || lower.contains("_spec.")).into()
}

/// Baseline-aligned `hotspots` aspect: CALLS in-degree only, over
/// Function/Test symbol nodes (this crate's closest equivalent to the
/// baseline's Function/Method label pair), excluding symbols whose
/// declaring file looks like a test file, ranked descending by
/// `fan_in`, capped at `limit`. Ties broken by node id for determinism
/// (the baseline's `qsort` documents no tie-break at all -- this crate
/// never leaves an ordering undefined).
fn hotspot_entries(
    graph: &CodeGraph,
    scope: ArchitectureScope<'_>,
    limit: ArchitectureHotspotLimit,
) -> Vec<HotspotEntry> {
    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_names: Vec<(ParsedSymbolName, ArchitectureNodeId)> = graph
        .symbol_nodes()
        .map(|s| (s.name.as_str().into(), s.id.as_str().into()))
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

    let mut fan_in: BTreeMap<ArchitectureNodeId, usize> = BTreeMap::new();
    for call in graph.calls() {
        let callee = ParsedCallee::from(call.callee.as_str());
        let Some(to_symbol_id) = resolve_callee(&callee, &symbol_names) else {
            continue;
        };
        if !is_function_like.contains(to_symbol_id.as_str()) {
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
            if !scope_matches!(scope, ArchitecturePath::from(file_path))
                || bool::from(looks_like_test_path(ArchitecturePath::from(file_path)))
            {
                return None;
            }
            Some(HotspotEntry {
                // CLONE-JUSTIFICATION: the report outlives this borrowed graph
                // traversal and therefore owns its exported symbol text.
                name: s.name.retained().into(),
                node_id: s.id.retained().into(),
                fan_in: fan_in
                    .get(&ArchitectureNodeId::from(s.id.as_str()))
                    .copied()
                    .unwrap_or(0)
                    .into(),
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        b.fan_in
            .cmp(&a.fan_in)
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    entries.truncate(limit.get());
    entries
}

/// The original hotspot metric scoped to nodes declared under `scope`.
///
/// [`CodeAdjacency::hotspots`] ranks a complete graph, so a scoped request
/// must rank every node before removing nodes outside the requested path.
/// Truncating first would let out-of-scope nodes consume the caller's limit
/// and return too few in-scope results.
fn scoped_hotspots(
    graph: &CodeGraph,
    scope: ArchitectureScope<'_>,
    limit: ArchitectureHotspotLimit,
) -> Vec<crate::analysis::HotspotScore> {
    if scope.prefix.is_none() {
        return CodeAdjacency::build(graph).hotspots(limit.get());
    }

    let mut node_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|file| (file.id.as_str(), file.rel_path.as_str()))
        .collect();
    node_path_by_id.extend(graph.symbol_nodes().map(|symbol| {
        let path = symbol
            .file_id
            .strip_prefix("file:")
            .unwrap_or(symbol.file_id.as_str());
        (symbol.id.as_str(), path)
    }));

    CodeAdjacency::build(graph)
        .hotspots(usize::MAX)
        .into_iter()
        .filter(|score| {
            node_path_by_id
                .get(score.node_id.as_str())
                .is_some_and(|path| scope_matches!(scope, ArchitecturePath::from(*path)))
        })
        .take(limit.get())
        .collect()
}

/// One directory's entry in [`FileTree`]: its own file/symbol counts
/// (direct children only) plus nested subdirectories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTreeNode {
    /// Repo-relative directory path, `"."` for the repo root.
    pub dir: ArchitectureReportPath,
    /// Files directly in this directory (not in a subdirectory).
    pub direct_file_count: ArchitectureItemCount,
    /// Symbols belonging to files directly in this directory.
    pub direct_symbol_count: ArchitectureItemCount,
    /// Files under this directory and every subdirectory, recursively.
    pub total_file_count: ArchitectureItemCount,
    /// Symbols under this directory and every subdirectory, recursively.
    pub total_symbol_count: ArchitectureItemCount,
    pub children: Vec<FileTreeNode>,
}

/// The hierarchical directory tree, rooted at `"."`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTree {
    pub root: FileTreeNode,
}

/// Baseline-aligned rule-based layer category (baseline tool schema doc
/// Â§7.2: `layers` -> `[{name, layer, reason}]`, `layer` in `{entry,
/// api, core, leaf, internal}` via "a rule-based classifier on
/// fan_in/fan_out + route/entry-point presence"). The baseline's exact
/// numeric thresholds are documented UNVERIFIED (no literal traced) --
/// this is a from-scratch, documented classifier matching the
/// *described* rule shape, not a byte-for-byte port of an unknown
/// constant.
/// One section's classification: [`CrateSection::name`], its
/// [`LayerCategory`], and a short human-readable reason string (the
/// baseline's `reason` field, per Â§7.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerClassification {
    pub name: ArchitectureName,
    pub layer: LayerCategory,
    pub reason: ArchitectureReason,
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
    pub languages: Option<Vec<(ArchitectureLanguage, ArchitectureItemCount)>>,
    pub packages: Option<Vec<PackageSection>>,
    pub entry_points: Option<Vec<EntryPoint>>,
    pub hotspots: Option<Vec<crate::analysis::HotspotScore>>,
    /// Baseline-aligned CALLS-fan-in-only hotspot ranking (baseline
    /// schema doc Â§7.3), populated alongside `hotspots` whenever
    /// [`Aspect::Hotspots`] is requested. `hotspots` above stays the
    /// original X06.3 total-degree metric for back-compat; this field
    /// is the parity-aligned one new callers should prefer for
    /// `get_architecture` behavioral parity.
    pub hotspot_entries: Option<Vec<HotspotEntry>>,
    pub boundaries: Option<Vec<Boundary>>,
    pub layers: Option<LayeringResult>,
    /// Baseline-aligned rule-based layer classification (baseline
    /// schema doc Â§7.2), populated alongside `layers` whenever
    /// [`Aspect::Layers`] is requested. `layers` above stays the
    /// topological/cycle-detecting ordering this pack's own hard tests
    /// require (the baseline's classifier has no cycle-detection
    /// concept at all); this field is the additional baseline-shaped
    /// view.
    pub layer_classification: Option<Vec<LayerClassification>>,
    pub file_tree: Option<FileTree>,
    pub clusters: Option<ClusteringResult>,
    /// Baseline-aligned per-cluster cohesion (baseline schema doc Â§7.4:
    /// `cohesion = internal_edges / (internal_edges + boundary_edges)`,
    /// ranked by descending member count), populated alongside
    /// `clusters` whenever [`Aspect::Clusters`] is requested. Kept
    /// separate from [`clustering::Cluster`] itself (rather than a new
    /// field there) so that struct's `Eq` derive is undisturbed by a
    /// floating-point field.
    pub cluster_cohesion: Option<Vec<ClusterCohesion>>,
}

/// One cluster's cohesion score (baseline schema doc Â§7.4): the
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
    pub cluster_id: ArchitectureClusterId,
    pub member_count: ArchitectureItemCount,
    pub cohesion: ArchitectureCohesion,
}

/// Converts dependency edges into their stable MCP JSON representation.
pub(crate) fn dependency_edges_json(edges: Vec<DependencyEdge>) -> Value {
    json!(edges
        .into_iter()
        .map(|edge| json!({
            "from": edge.from.0,
            "to": edge.to.0,
            "count": edge.count.0,
        }))
        .collect::<Vec<_>>())
}

/// Converts cross-section boundaries into their stable MCP JSON representation.
pub(crate) fn boundaries_json(boundaries: Vec<Boundary>) -> Value {
    json!(boundaries
        .into_iter()
        .map(|boundary| json!({
            "from": boundary.from.0,
            "to": boundary.to.0,
            "callCount": boundary.call_count.0,
        }))
        .collect::<Vec<_>>())
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
    path_prefix: Option<ArchitectureReportPath>,
    hotspot_limit: impl Into<ArchitectureHotspotLimit>,
    max_iterations: impl Into<ArchitectureMaxIterations>,
) -> ArchitectureReport {
    let hotspot_limit = hotspot_limit.into();
    let max_iterations = max_iterations.into();
    let scope_prefix = path_prefix.filter(|prefix| !prefix.is_empty());
    let scope = ArchitectureScope {
        prefix: scope_prefix
            .as_ref()
            .map(|prefix| ArchitecturePath::from(prefix.as_str())),
    };
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
            Some(filtered_clusters(graph, scope, max_iterations))
        } else {
            None
        };

    if wanted.contains(&Aspect::Overview) {
        report.overview = Some(ArchitectureOverview {
            sections: ArchitectureSections(crate_sections(graph, scope)),
            hotspots: ArchitectureHotspots(scoped_hotspots(graph, scope, hotspot_limit)),
            language_counts: ArchitectureLanguageCounts(language_counts(graph, scope)),
            total_files: ArchitectureFileCount(
                graph
                    .file_nodes()
                    .filter(|f| scope_matches!(scope, ArchitecturePath::from(&f.rel_path)))
                    .count(),
            ),
            total_symbols: ArchitectureSymbolCount(symbol_count_under(graph, scope).into()),
        });
    }
    if wanted.contains(&Aspect::Structure) {
        report.structure = Some(crate_sections(graph, scope));
    }
    if wanted.contains(&Aspect::Dependencies) {
        report.dependencies = Some(dependency_edges(graph, scope));
    }
    if wanted.contains(&Aspect::Routes) {
        // Baseline-aligned cap (baseline schema doc Â§7.2: `routes` ->
        // capped at 20). Applied only at this response-building site,
        // never inside `route_entries` itself, so other callers
        // (`layer_classification`'s route-presence detection,
        // `entry_points`'s RouteHandler detection) still see every
        // route regardless of the aspect-level display cap.
        let mut routes = route_entries(graph, scope);
        routes.truncate(20);
        report.routes = Some(routes);
    }
    if wanted.contains(&Aspect::Languages) {
        report.languages = Some(language_counts(graph, scope));
    }
    if wanted.contains(&Aspect::Packages) {
        report.packages = Some(package_sections(graph, scope));
    }
    if wanted.contains(&Aspect::EntryPoints) {
        report.entry_points = Some(entry_points(graph, scope));
    }
    if wanted.contains(&Aspect::Hotspots) {
        report.hotspots = Some(scoped_hotspots(graph, scope, hotspot_limit));
        report.hotspot_entries = Some(hotspot_entries(graph, scope, hotspot_limit));
    }
    if wanted.contains(&Aspect::Boundaries) {
        report.boundaries = Some(boundaries(graph, scope));
    }
    if wanted.contains(&Aspect::Layers) {
        if let Some(clusters) = &clustering_result {
            report.layers = Some(layering(clusters));
        }
        report.layer_classification = Some(layer_classification(graph, scope));
    }
    if wanted.contains(&Aspect::FileTree) {
        report.file_tree = Some(file_tree(graph, scope));
    }
    if wanted.contains(&Aspect::Clusters) {
        if let Some(clusters) = &clustering_result {
            report.cluster_cohesion = Some(cluster_cohesion(clusters));
        }
        report.clusters = clustering_result;
    }

    report
}

fn symbol_count_under(graph: &CodeGraph, scope: ArchitectureScope<'_>) -> ArchitectureItemCount {
    graph
        .symbol_nodes()
        .filter(|s| {
            let rel_path = s.file_id.strip_prefix("file:").unwrap_or(&s.file_id);
            scope_matches!(scope, ArchitecturePath::from(rel_path))
        })
        .count()
        .into()
}

fn crate_sections(graph: &CodeGraph, scope: ArchitectureScope<'_>) -> Vec<CrateSection> {
    let mut sections: BTreeMap<ArchitectureSectionGroup, CrateSection> = BTreeMap::new();

    for file in graph.file_nodes() {
        if !scope_matches!(scope, ArchitecturePath::from(&file.rel_path)) {
            continue;
        }
        let crate_name =
            ArchitectureSectionGroup::from_path(ArchitecturePath::from(&file.rel_path));
        let section = sections
            // CLONE-JUSTIFICATION: the typed key indexes the aggregation map
            // while the report independently owns its output text.
            .entry(crate_name.retained())
            .or_insert_with(|| CrateSection {
                name: crate_name.0.retained().into(),
                file_count: 0usize.into(),
                symbol_count: 0usize.into(),
                rel_paths: Vec::new(),
            });
        section.file_count += 1;
        // CLONE-JUSTIFICATION: CrateSection is an owned report result while
        // the source path remains borrowed from the indexed graph.
        section.rel_paths.push(file.rel_path.retained().into());
    }

    for symbol in graph.symbol_nodes() {
        let rel_path = symbol
            .file_id
            .strip_prefix("file:")
            .unwrap_or(&symbol.file_id);
        if !scope_matches!(scope, ArchitecturePath::from(rel_path)) {
            continue;
        }
        let crate_name = ArchitectureSectionGroup::from_path(ArchitecturePath::from(rel_path));
        if let Some(section) = sections.get_mut(&crate_name) {
            section.symbol_count += 1;
        }
    }

    sections.into_values().collect()
}

fn language_counts(
    graph: &CodeGraph,
    scope: ArchitectureScope<'_>,
) -> Vec<(ArchitectureLanguage, ArchitectureItemCount)> {
    let mut language_counts: BTreeMap<ArchitectureLanguage, ArchitectureItemCount> =
        BTreeMap::new();
    for file in graph.file_nodes() {
        if !scope_matches!(scope, ArchitecturePath::from(&file.rel_path)) {
            continue;
        }
        *language_counts
            .entry(format!("{:?}", file.language).into())
            .or_default() += 1;
    }
    language_counts.into_iter().collect()
}

fn route_entries(graph: &CodeGraph, scope: ArchitectureScope<'_>) -> Vec<RouteEntry> {
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
            if !scope_matches!(scope, ArchitecturePath::from(declared_in)) {
                return None;
            }
            Some(RouteEntry {
                // CLONE-JUSTIFICATION: route rows are retained in the owned
                // architecture report after this borrowed graph traversal.
                method: r.method.retained().into(),
                path: r.path.retained().into(),
                declared_in: declared_in.retained_display().into(),
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
fn is_manifest_file(path: ArchitecturePath<'_>) -> ArchitecturePathMatch {
    let name = path.as_str().rsplit('/').next().unwrap_or(path.as_str());
    (name == "Cargo.toml" || name == "package.json").into()
}

fn package_sections(graph: &CodeGraph, scope: ArchitectureScope<'_>) -> Vec<PackageSection> {
    let manifests: Vec<ArchitecturePath<'_>> = graph
        .file_nodes()
        .filter(|f| {
            scope_matches!(scope, ArchitecturePath::from(&f.rel_path))
                && bool::from(is_manifest_file(ArchitecturePath::from(&f.rel_path)))
        })
        .map(|f| ArchitecturePath::from(&f.rel_path))
        .collect();

    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_names: Vec<(ParsedSymbolName, ArchitectureNodeId)> = graph
        .symbol_nodes()
        .map(|s| (s.name.as_str().into(), s.id.as_str().into()))
        .collect();
    let symbol_file_by_id: BTreeMap<&str, &str> = graph
        .symbol_nodes()
        .map(|s| (s.id.as_str(), s.file_id.as_str()))
        .collect();

    let mut sections: Vec<PackageSection> = Vec::new();
    for manifest in manifests {
        let dir = ArchitectureDirectory::from_file_path(manifest);
        let members: Vec<String> = graph
            .file_nodes()
            .filter(|f| {
                scope_matches!(scope, ArchitecturePath::from(&f.rel_path))
                    && f.rel_path != manifest.as_str()
                    && bool::from(dir.contains_path(ArchitecturePath::from(&f.rel_path)))
            })
            // CLONE-JUSTIFICATION: package membership is returned as owned
            // report data independent of the indexed graph's file nodes.
            .map(|f| f.rel_path.retained())
            .collect();

        // Package-scoped, CALLS-only fan_in/fan_out (baseline-aligned
        // shape; see PackageSection docs for why these are computed
        // rather than left at the baseline's own always-zero fallback path).
        let (mut fan_in, mut fan_out) = (0usize, 0usize);
        for call in graph.calls() {
            let Some(&from_path) = file_path_by_id.get(call.from_file_id.as_str()) else {
                continue;
            };
            if !scope_matches!(scope, ArchitecturePath::from(from_path)) {
                continue;
            }
            let from_inside = bool::from(dir.contains_path(ArchitecturePath::from(from_path)));
            let callee = ParsedCallee::from(call.callee.as_str());
            let Some(to_symbol_id) = resolve_callee(&callee, &symbol_names) else {
                continue;
            };
            let Some(&to_file_id) = symbol_file_by_id.get(to_symbol_id.as_str()) else {
                continue;
            };
            let Some(&to_path) = file_path_by_id.get(to_file_id) else {
                continue;
            };
            if !scope_matches!(scope, ArchitecturePath::from(to_path)) {
                continue;
            }
            let to_inside = bool::from(dir.contains_path(ArchitecturePath::from(to_path)));
            match (from_inside, to_inside) {
                (true, false) => fan_out += 1,
                (false, true) => fan_in += 1,
                _ => {}
            }
        }

        sections.push(PackageSection {
            // ALLOC-JUSTIFICATION: PackageSection crosses the architecture
            // report boundary and must own its serialized directory text.
            dir: dir.0.into(),
            manifest_rel_path: manifest.as_str().retained().into(),
            member_file_count: members.len().into(),
            member_rel_paths: members.into_iter().map(Into::into).collect(),
            fan_in: fan_in.into(),
            fan_out: fan_out.into(),
        });
    }
    sections.sort_by(|a, b| a.manifest_rel_path.cmp(&b.manifest_rel_path));
    sections
}

fn entry_points(graph: &CodeGraph, scope: ArchitectureScope<'_>) -> Vec<EntryPoint> {
    let mut entries: Vec<EntryPoint> = Vec::new();
    for file in graph.file_nodes() {
        if !scope_matches!(scope, ArchitecturePath::from(&file.rel_path)) {
            continue;
        }
        let name = file.rel_path.rsplit('/').next().unwrap_or(&file.rel_path);
        if name == "main.rs" {
            entries.push(EntryPoint {
                // CLONE-JUSTIFICATION: the public entry-point report owns
                // this path after the borrowed graph traversal completes.
                rel_path: file.rel_path.retained().into(),
                kind: EntryPointKind::BinaryMain,
            });
        } else if name == "lib.rs" {
            entries.push(EntryPoint {
                // CLONE-JUSTIFICATION: the public entry-point report owns
                // this path after the borrowed graph traversal completes.
                rel_path: file.rel_path.retained().into(),
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
            if scope_matches!(scope, ArchitecturePath::from(rel_path)) {
                route_files.insert(rel_path);
            }
        }
    }
    for rel_path in route_files {
        entries.push(EntryPoint {
            rel_path: rel_path.retained_display().into(),
            kind: EntryPointKind::RouteHandler,
        });
    }
    entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    entries
}

/// Resolve every import/call edge to a (from-section, to-section) pair
/// using the same best-effort suffix/name matching
/// [`crate::analysis::CodeAdjacency`] documents, and tally counts.
fn dependency_edges(graph: &CodeGraph, scope: ArchitectureScope<'_>) -> Vec<DependencyEdge> {
    let file_paths: Vec<(ArchitecturePath<'_>, ArchitectureNodeId)> = graph
        .file_nodes()
        .filter(|f| scope_matches!(scope, ArchitecturePath::from(&f.rel_path)))
        .map(|f| (ArchitecturePath::from(&f.rel_path), f.id.as_str().into()))
        .collect();
    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_names: Vec<(ParsedSymbolName, ArchitectureNodeId)> = graph
        .symbol_nodes()
        .map(|s| (s.name.as_str().into(), s.id.as_str().into()))
        .collect();
    let symbol_file_by_id: BTreeMap<&str, &str> = graph
        .symbol_nodes()
        .map(|s| (s.id.as_str(), s.file_id.as_str()))
        .collect();

    let mut counts: BTreeMap<(ArchitectureSectionGroup, ArchitectureSectionGroup), usize> =
        BTreeMap::new();

    for import in graph.imports() {
        let Some(&from_path) = file_path_by_id.get(import.from_file_id.as_str()) else {
            continue;
        };
        if !scope_matches!(scope, ArchitecturePath::from(from_path)) {
            continue;
        }
        if let Some(to_path) = resolve_module_path(
            &ParsedModulePath::from(import.module_path.as_str()),
            &file_paths,
        ) {
            let from_section =
                ArchitectureSectionGroup::from_path(ArchitecturePath::from(from_path));
            let to_section = ArchitectureSectionGroup::from_path(to_path);
            if from_section != to_section {
                *counts.entry((from_section, to_section)).or_insert(0) += 1;
            }
        }
    }

    for call in graph.calls() {
        let Some(&from_path) = file_path_by_id.get(call.from_file_id.as_str()) else {
            continue;
        };
        if !scope_matches!(scope, ArchitecturePath::from(from_path)) {
            continue;
        }
        let callee = ParsedCallee::from(call.callee.as_str());
        if let Some(to_symbol_id) = resolve_callee(&callee, &symbol_names) {
            if let Some(&to_file_id) = symbol_file_by_id.get(to_symbol_id.as_str()) {
                if let Some(&to_path) = file_path_by_id.get(to_file_id) {
                    if scope_matches!(scope, ArchitecturePath::from(to_path)) {
                        let from_section =
                            ArchitectureSectionGroup::from_path(ArchitecturePath::from(from_path));
                        let to_section =
                            ArchitectureSectionGroup::from_path(ArchitecturePath::from(to_path));
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
        .map(|((from, to), count)| DependencyEdge {
            from: ArchitectureSectionId::from_key(from),
            to: ArchitectureSectionId::from_key(to),
            count: CrossSectionEdgeCount(count),
        })
        .collect()
}

fn resolve_module_path<'a>(
    module_path: &ParsedModulePath,
    file_paths: &[(ArchitecturePath<'a>, ArchitectureNodeId)],
) -> Option<ArchitecturePath<'a>> {
    let needle = module_path
        .as_str()
        .trim_start_matches("./")
        .trim_start_matches("../");
    let last_segment = needle.rsplit(['/', ':', '.']).next().unwrap_or(needle);
    if last_segment.is_empty() {
        return None;
    }
    file_paths
        .iter()
        .find(|(rel_path, _)| {
            let stem = rel_path
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or(rel_path.as_str());
            let stem = stem.split('.').next().unwrap_or(stem);
            stem == last_segment || rel_path.as_str().ends_with(last_segment)
        })
        .map(|(rel_path, _)| *rel_path)
}

fn resolve_callee(
    callee: &ParsedCallee,
    symbol_names: &[(ParsedSymbolName, ArchitectureNodeId)],
) -> Option<ArchitectureNodeId> {
    let last_segment = callee
        .as_str()
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(callee.as_str());
    symbol_names
        .iter()
        .find(|(name, _)| name.as_str() == callee.as_str() || name.as_str() == last_segment)
        .map(|(_, id)| id.as_str().into())
}

/// Baseline-aligned `boundaries` aspect (baseline tool schema doc Â§7.2:
/// `[{from, to, call_count}]`, "cross-package CALLS edge counts") --
/// directed, CALLS-edges only (never imports), matching the baseline's
/// semantics rather than [`dependency_edges`]'s broader import+call
/// mix used by [`Aspect::Dependencies`].
fn boundaries(graph: &CodeGraph, scope: ArchitectureScope<'_>) -> Vec<Boundary> {
    let file_path_by_id: BTreeMap<&str, &str> = graph
        .file_nodes()
        .map(|f| (f.id.as_str(), f.rel_path.as_str()))
        .collect();
    let symbol_names: Vec<(ParsedSymbolName, ArchitectureNodeId)> = graph
        .symbol_nodes()
        .map(|s| (s.name.as_str().into(), s.id.as_str().into()))
        .collect();
    let symbol_file_by_id: BTreeMap<&str, &str> = graph
        .symbol_nodes()
        .map(|s| (s.id.as_str(), s.file_id.as_str()))
        .collect();

    let mut counts: BTreeMap<(ArchitectureSectionGroup, ArchitectureSectionGroup), usize> =
        BTreeMap::new();
    for call in graph.calls() {
        let Some(&from_path) = file_path_by_id.get(call.from_file_id.as_str()) else {
            continue;
        };
        if !scope_matches!(scope, ArchitecturePath::from(from_path)) {
            continue;
        }
        let callee = ParsedCallee::from(call.callee.as_str());
        let Some(to_symbol_id) = resolve_callee(&callee, &symbol_names) else {
            continue;
        };
        let Some(&to_file_id) = symbol_file_by_id.get(to_symbol_id.as_str()) else {
            continue;
        };
        let Some(&to_path) = file_path_by_id.get(to_file_id) else {
            continue;
        };
        if !scope_matches!(scope, ArchitecturePath::from(to_path)) {
            continue;
        }
        let from_section = ArchitectureSectionGroup::from_path(ArchitecturePath::from(from_path));
        let to_section = ArchitectureSectionGroup::from_path(ArchitecturePath::from(to_path));
        if from_section != to_section {
            *counts.entry((from_section, to_section)).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .map(|((from, to), call_count)| Boundary {
            from: ArchitectureSectionId::from_key(from),
            to: ArchitectureSectionId::from_key(to),
            call_count: CrossSectionEdgeCount(call_count),
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
    scope: ArchitectureScope<'_>,
    max_iterations: ArchitectureMaxIterations,
) -> ClusteringResult {
    let result = clustering::detect_clusters(graph, max_iterations.get());
    if scope.prefix.is_none() {
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
            Some(path) => scope_matches!(scope, ArchitecturePath::from(path)),
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
                // CLONE-JUSTIFICATION: the retained-id filter owns its set
                // while the moved cluster must keep its original identifier.
                kept_cluster_ids.insert(cluster.id.retained_display());
                Some(cluster)
            }
        })
        .collect();

    let inter_cluster_edges = result
        .inter_cluster_edges
        .into_iter()
        .filter(|e| {
            kept_cluster_ids.contains(e.from_cluster.as_str())
                && kept_cluster_ids.contains(e.to_cluster.as_str())
        })
        .collect();

    ClusteringResult {
        clusters,
        inter_cluster_edges,
    }
}

/// Baseline-aligned per-cluster cohesion (baseline schema doc Â§7.4:
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
            .or_insert(0) += edge.count.get();
        *boundary_edges.entry(edge.to_cluster.as_str()).or_insert(0) += edge.count.get();
    }

    let mut entries: Vec<ClusterCohesion> = result
        .clusters
        .iter()
        .map(|cluster| {
            let member_count = cluster.size();
            let internal_edges = member_count.get().saturating_sub(1);
            let boundary = boundary_edges
                .get(cluster.id.as_str())
                .copied()
                .unwrap_or(0);
            let denom = internal_edges + boundary;
            let cohesion = if denom == 0 {
                1.0
            } else {
                crate::owned_boundary::usize_to_f64(internal_edges)
                    / crate::owned_boundary::usize_to_f64(denom)
            };
            ClusterCohesion {
                // CLONE-JUSTIFICATION: cohesion is an owned report view while
                // cluster identifiers remain owned by the clustering result.
                cluster_id: cluster.id.as_str().into(),
                member_count: member_count.get().into(),
                cohesion: cohesion.into(),
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
/// doc Â§7.2: `[{name, layer, reason}]`, categories `entry|api|core|
/// leaf|internal` "via a rule-based classifier on fan_in/fan_out +
/// route/entry-point presence"). Operates per [`CrateSection`] (see
/// [`crate_sections`]), using [`boundaries`] (CALLS-only, directed) for
/// fan_in/fan_out counts and [`entry_points`]/[`route_entries`] for
/// entry-surface detection. Deterministic: sections are visited in
/// [`crate_sections`]' `BTreeMap`-derived order and every count is an
/// exact sum, no sampling.
fn layer_classification(
    graph: &CodeGraph,
    scope: ArchitectureScope<'_>,
) -> Vec<LayerClassification> {
    let sections = crate_sections(graph, scope);
    let cross_edges = boundaries(graph, scope);
    let entries = entry_points(graph, scope);
    let routes = route_entries(graph, scope);

    let mut fan_in: BTreeMap<ArchitectureSectionGroup, usize> = BTreeMap::new();
    let mut fan_out: BTreeMap<ArchitectureSectionGroup, usize> = BTreeMap::new();
    for edge in &cross_edges {
        // CLONE-JUSTIFICATION: independent inbound and outbound aggregation
        // maps each own their typed key after the edge view is dropped.
        *fan_out
            .entry(ArchitectureSectionGroup(edge.from.0.retained()))
            .or_insert(0) += edge.call_count.0;
        // CLONE-JUSTIFICATION: the inbound map independently owns its key.
        *fan_in
            .entry(ArchitectureSectionGroup(edge.to.0.retained()))
            .or_insert(0) += edge.call_count.0;
    }

    let mut has_entry_point: BTreeSet<ArchitectureSectionGroup> = BTreeSet::new();
    for entry in &entries {
        has_entry_point.insert(ArchitectureSectionGroup::from_path(ArchitecturePath::from(
            entry.rel_path.as_str(),
        )));
    }
    let mut has_route: BTreeSet<ArchitectureSectionGroup> = BTreeSet::new();
    for route in &routes {
        has_route.insert(ArchitectureSectionGroup::from_path(ArchitecturePath::from(
            route.declared_in.as_str(),
        )));
    }

    sections
        .iter()
        .map(|section| {
            let section_key = ArchitectureSectionGroup(section.name.as_str().retained());
            // CLONE-JUSTIFICATION: the public report owns its section text;
            // aggregation uses a separate typed key for lookups.
            let name = section.name.retained();
            let this_fan_in = fan_in.get(&section_key).copied().unwrap_or(0);
            let this_fan_out = fan_out.get(&section_key).copied().unwrap_or(0);
            let is_entry = has_entry_point.contains(&section_key);
            let is_api = has_route.contains(&section_key);

            let (layer, reason) = if is_api {
                (
                    LayerCategory::Api,
                    "declares at least one route".retained_display(),
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
                    "no cross-section calls in either direction".retained_display(),
                )
            };

            LayerClassification {
                name,
                layer,
                reason: reason.into(),
            }
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
#[doc(hidden)]
pub fn layering(clusters: &ClusteringResult) -> LayeringResult {
    // CLONE-JUSTIFICATION: layering mutates independent working sets while
    // preserving the clustering result for the caller's other report views.
    let all_ids: BTreeSet<String> = clusters
        .clusters
        .iter()
        .map(|c| c.id.retained_display())
        .collect();
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
        .map(|id| (id.retained(), BTreeSet::new()))
        .collect();
    for edge in &clusters.inter_cluster_edges {
        // CLONE-JUSTIFICATION: dependency adjacency owns both endpoint ids
        // independently of the immutable clustering-result edge list.
        remaining_deps
            .entry(edge.from_cluster.retained_display())
            .or_default()
            .insert(edge.to_cluster.retained_display());
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
            // CLONE-JUSTIFICATION: each output layer owns its ids while the
            // dependency map remains available for later layers.
            .map(|(id, _)| id.retained())
            .collect();
        ready.sort();

        if ready.is_empty() {
            break;
        }

        for id in &ready {
            // CLONE-JUSTIFICATION: `placed` retains completed ids while
            // `ready` is moved into the returned layer below.
            placed.insert(id.retained());
        }
        layers.push(Layer {
            index: layer_index.into(),
            cluster_ids: ready.into_iter().map(Into::into).collect(),
        });
        layer_index += 1;
    }

    let cycle_cluster_ids: Vec<String> = all_ids
        .into_iter()
        .filter(|id| !placed.contains(id))
        .collect();

    LayeringResult {
        layers,
        cycle_cluster_ids: cycle_cluster_ids.into_iter().map(Into::into).collect(),
    }
}

fn file_tree(graph: &CodeGraph, scope: ArchitectureScope<'_>) -> FileTree {
    // dir path -> (direct_file_count, direct_symbol_count).
    let mut direct_files: BTreeMap<ArchitectureDirectory, ArchitectureItemCount> = BTreeMap::new();
    let mut direct_symbols: BTreeMap<ArchitectureDirectory, ArchitectureItemCount> =
        BTreeMap::new();
    let mut all_dirs: BTreeSet<ArchitectureDirectory> = BTreeSet::new();
    all_dirs.insert(ArchitectureDirectory::root());

    let file_dir_by_id: BTreeMap<&str, ArchitectureDirectory> = graph
        .file_nodes()
        .filter(|f| scope_matches!(scope, ArchitecturePath::from(&f.rel_path)))
        .map(|f| {
            (
                f.id.as_str(),
                ArchitectureDirectory::from_file_path(ArchitecturePath::from(&f.rel_path)),
            )
        })
        .collect();

    for file in graph.file_nodes() {
        if !scope_matches!(scope, ArchitecturePath::from(&file.rel_path)) {
            continue;
        }
        let dir = ArchitectureDirectory::from_file_path(ArchitecturePath::from(&file.rel_path));
        // CLONE-JUSTIFICATION: `direct_files` owns its key while the same
        // directory value is also required to register its ancestors.
        *direct_files.entry(dir.retained()).or_default() += 1;
        register_ancestors(&mut all_dirs, &dir);
    }

    for symbol in graph.symbol_nodes() {
        if let Some(dir) = file_dir_by_id.get(symbol.file_id.as_str()) {
            // CLONE-JUSTIFICATION: direct-symbol aggregation owns its key;
            // the file-id index retains the directory for later symbols.
            *direct_symbols.entry(dir.retained()).or_default() += 1;
        }
    }

    let root = build_tree_node(
        &ArchitectureDirectory::root(),
        &all_dirs,
        &direct_files,
        &direct_symbols,
    );
    FileTree { root }
}

fn register_ancestors(all_dirs: &mut BTreeSet<ArchitectureDirectory>, dir: &ArchitectureDirectory) {
    // CLONE-JUSTIFICATION: the hierarchy set owns each directory key, while
    // the caller retains its typed directory for the direct-file aggregation.
    all_dirs.insert(dir.retained());
    let mut current = dir.retained();
    while let Some(parent) = current.parent() {
        all_dirs.insert(parent.retained());
        current = parent;
    }
}

fn direct_children<'a>(
    all_dirs: &'a BTreeSet<ArchitectureDirectory>,
    dir: &ArchitectureDirectory,
) -> Vec<&'a ArchitectureDirectory> {
    all_dirs
        .iter()
        .filter(|candidate| {
            if *candidate == dir {
                return false;
            }
            // A direct child's own `dir_of(...)` is exactly `dir` (both
            // for the root, where a top-level candidate like `"crates"`
            // has no `/` and produces the typed root directory, and for any
            // nested directory).
            ArchitectureDirectory::from_file_path(ArchitecturePath::from(&candidate.0)) == *dir
        })
        .collect()
}

fn build_tree_node(
    dir: &ArchitectureDirectory,
    all_dirs: &BTreeSet<ArchitectureDirectory>,
    direct_files: &BTreeMap<ArchitectureDirectory, ArchitectureItemCount>,
    direct_symbols: &BTreeMap<ArchitectureDirectory, ArchitectureItemCount>,
) -> FileTreeNode {
    let direct_file_count = direct_files
        .get(dir)
        .copied()
        .unwrap_or_else(|| ArchitectureItemCount::try_new(0));
    let direct_symbol_count = direct_symbols
        .get(dir)
        .copied()
        .unwrap_or_else(|| ArchitectureItemCount::try_new(0));

    let children: Vec<FileTreeNode> = direct_children(all_dirs, dir)
        .into_iter()
        .map(|child_dir| build_tree_node(child_dir, all_dirs, direct_files, direct_symbols))
        .collect();

    let total_file_count = usize::from(direct_file_count)
        + children
            .iter()
            .map(|c| usize::from(c.total_file_count))
            .sum::<usize>();
    let total_symbol_count = usize::from(direct_symbol_count)
        + children
            .iter()
            .map(|c| usize::from(c.total_symbol_count))
            .sum::<usize>();

    FileTreeNode {
        // ALLOC-JUSTIFICATION: the public report owns its serialized
        // directory text independently of the internal typed aggregation key.
        dir: dir.0.retained().into(),
        direct_file_count,
        direct_symbol_count,
        total_file_count: total_file_count.into(),
        total_symbol_count: total_symbol_count.into(),
        children,
    }
}
