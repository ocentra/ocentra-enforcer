#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod desktop_scan_history;
mod memory_commands;
mod project_settings;
mod project_registry;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use enforcer_literal_scan::{language_registry, LanguageSpec};
use enforcer_memory::artifacts::{GraphSnapshot, GraphSymbolKindSnapshot};
use enforcer_memory::code_graph::CodeGraph;
use enforcer_memory::ids::repo_root;
use enforcer_memory::store::{sqlite::OperationalGraph, Store};
use enforcer_proof::read_model::{read_project_proof_snapshot, ProjectProofSnapshot};
use globset::{GlobBuilder, GlobSetBuilder};
use desktop_scan_history::{
    desktop_scan_run_id, desktop_scan_run_path, load_cached_scan, load_desktop_scan_history,
    load_desktop_scan_run, persist_desktop_report, DesktopReportPayload,
};
use memory_commands::{
    create_memory_index, load_memory_summary, memory_index_status, search_memory_graph,
};
use project_settings::{
    load_project_settings, load_scan_scope_settings, write_rule_override,
    write_scan_scope_settings,
};
use project_registry::{
    discover_desktop_project_worktrees, git_value, load_desktop_projects,
    preview_desktop_project_registration, register_desktop_project,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Tier;
use enforcer_rules::registry::{FixtureRef, RuleRecord, RuleRegistry, ValidatorRef};
use enforcer_rules::waiver::WaiverDate;
use enforcer_ui::actions::file_rule_waiver::{upsert_file_rule_waiver, FileRuleWaiverRequest};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopStatus {
    app: &'static str,
    shell: &'static str,
    binding_mode: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineCapabilityPayload {
    capabilities: Vec<EngineCapability>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineCapability {
    id: &'static str,
    domain: &'static str,
    title: &'static str,
    state: &'static str,
    source: &'static str,
    controls: &'static str,
    missing: &'static str,
    target: Option<EngineCapabilityTarget>,
    workpacks: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EngineCapabilityTarget {
    mode: &'static str,
    workspace: &'static str,
    subview: Option<&'static str>,
    project_context: &'static str,
}

fn project_target(workspace: &'static str) -> EngineCapabilityTarget {
    EngineCapabilityTarget {
        mode: "project",
        workspace,
        subview: None,
        project_context: "required",
    }
}

fn global_project_target(workspace: &'static str) -> EngineCapabilityTarget {
    EngineCapabilityTarget {
        mode: "project",
        workspace,
        subview: None,
        project_context: "none",
    }
}

fn hub_target(subview: Option<&'static str>) -> EngineCapabilityTarget {
    EngineCapabilityTarget {
        mode: "hub",
        workspace: "hub",
        subview,
        project_context: "none",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessDiscoveryPayload {
    harnesses: Vec<enforcer_install::detect::DetectedHarness>,
    runtime: &'static str,
    verification: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkpackIndexPayload {
    source_path: String,
    rows: Vec<WorkpackIndexRow>,
    status_counts: BTreeMap<String, usize>,
    caveat: &'static str,
}

#[derive(Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WorkpackIndexRow {
    id: String,
    title: String,
    status: String,
    track: String,
    owns: String,
    tier: String,
    dependencies: String,
    parallel_safe_with: String,
    source_path: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRuleCatalog {
    schema_version: u32,
    product_name: String,
    languages: Vec<String>,
    rules: Vec<DesktopRuleCatalogRule>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRuleCatalogRule {
    id: String,
    language: String,
    family: String,
    severity: String,
    title: String,
    snippet: String,
    lock_level: String,
    can_disable: bool,
    can_downgrade: bool,
    #[serde(default)]
    waivable: bool,
    requires_fail_fixture: bool,
    requires_pass_fixture: bool,
    applies_to: Vec<String>,
    triggers: Vec<String>,
    validator: String,
    doc: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectRuleCoverageRow {
    rule_id: String,
    language: String,
    scope: String,
    effective_severity: String,
    state: String,
    path_match_status: &'static str,
    matched_path_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectRuleCoveragePayload {
    detected_languages: Vec<String>,
    catalog_languages: Vec<String>,
    observed_without_catalog: Vec<String>,
    settings_status: String,
    rules: Vec<ProjectRuleCoverageRow>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecurityProfileFile {
    profile_name: String,
    required_test_categories: Vec<String>,
    invariants: Vec<String>,
    rules: Vec<SecurityProfileRule>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SecurityProfileRule {
    rule_id: String,
    tier: String,
    backed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SecurityProfilePayload {
    source_path: String,
    profile_name: String,
    required_test_categories: Vec<String>,
    invariants: Vec<String>,
    rules: Vec<SecurityProfileRule>,
    activated: bool,
    project_activation: String,
    caveat: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecurityActivationRequest {
    source_spec: String,
    owner: String,
    reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum LegacyAnalysisKind {
    TestDoctrine,
    UiLogicCoupling,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyAnalysisEnvelope {
    schema_version: u8,
    analysis_kind: LegacyAnalysisKind,
    report: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisRunMetadata {
    schema_version: u8,
    runtime: &'static str,
    source: &'static str,
    state: &'static str,
    generated_at: String,
    caveat: &'static str,
}

#[derive(Serialize)]
#[serde(tag = "analysisKind", rename_all = "kebab-case")]
enum LegacyAnalysisRunPayload {
    TestDoctrine {
        metadata: AnalysisRunMetadata,
        report: TestDoctrineReport,
    },
    UiLogicCoupling {
        metadata: AnalysisRunMetadata,
        report: UiLogicCouplingReport,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestDoctrineReport {
    root: String,
    caveat: String,
    nature: TestDoctrineNature,
    ci_config_files_found: Vec<String>,
    has_untracked_ci_files: bool,
    detected: BTreeMap<String, TestDoctrineDetection>,
    missing: Vec<TestDoctrineMissing>,
    ci_gaps: Vec<TestDoctrineCiGap>,
    summary: TestDoctrineSummary,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestDoctrineNature {
    languages: BTreeMap<String, usize>,
    is_web_api: bool,
    has_open_api_spec: bool,
    has_frontend_ui: bool,
    has_async_workers: bool,
    has_money_critical_surface: bool,
    money_critical_files: Vec<String>,
    has_multi_service_boundary: bool,
    multi_service_client_files: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestDoctrineDetection {
    label: String,
    present: bool,
    evidence: Vec<String>,
    relevant: bool,
    ci: TestDoctrineCiState,
    ci_including_untracked: Option<TestDoctrineCiState>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestDoctrineCiState {
    wired: bool,
    blocking: bool,
    evidence: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestDoctrineMissing {
    category: String,
    label: String,
    tier: String,
    reason: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestDoctrineCiGap {
    category: String,
    label: String,
    reason: String,
    ci_evidence: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestDoctrineSummary {
    categories_relevant: usize,
    categories_present: usize,
    categories_missing: usize,
    core_missing: usize,
    ci_gaps: usize,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiLogicCouplingReport {
    root: String,
    rule: UiLogicCouplingRule,
    caveat: String,
    findings: Vec<UiLogicCouplingFinding>,
    summary: UiLogicCouplingSummary,
    hard: Vec<UiLogicCouplingFinding>,
    info: Vec<UiLogicCouplingFinding>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiLogicCouplingRule {
    id: String,
    title: String,
    doc: String,
    aka: String,
    why: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiLogicCouplingFinding {
    file: String,
    kind: String,
    severity: String,
    source: String,
    binding: String,
    #[serde(default)]
    has_data_fetch_primitive: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiLogicCouplingSummary {
    total_findings: usize,
    hard_findings: usize,
    info_findings: usize,
    files_with_hard_findings: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessRunsPayload {
    root: String,
    storage: &'static str,
    runs: Vec<HarnessRunRow>,
    last_failure: Option<HarnessFailurePayload>,
    caveat: &'static str,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessRunRow {
    run_id: String,
    tool: String,
    language: Option<String>,
    command: Vec<String>,
    status: String,
    exit_code: i64,
    started_at: String,
    ended_at: String,
    diagnostic_count: usize,
    pinned: bool,
    storage_root: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessFailurePayload {
    run: HarnessRunRow,
    diagnostics: Vec<HarnessDiagnosticPayload>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessDiagnosticPayload {
    severity: String,
    rule_id: String,
    file: String,
    line: usize,
    message: String,
    source: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessRunDetailPayload {
    run: HarnessRunRow,
    diagnostics: Vec<HarnessDiagnosticPayload>,
    stdout: HarnessArtifactPayload,
    stderr: HarnessArtifactPayload,
    caveat: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HarnessArtifactPayload {
    available: bool,
    content: String,
    error: Option<String>,
}

const MAX_PROJECTION_NODES: usize = 2_400;
const MAX_PROJECTION_EDGES: usize = 7_200;
const MAX_PROJECTION_FILES: usize = 560;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphNodePayload {
    id: String,
    label: String,
    kind: String,
    path: String,
    line: usize,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphEdgePayload {
    from: String,
    to: String,
    label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphFolderAggregatePayload {
    path: String,
    files: usize,
    symbols: usize,
    calls: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphPayload {
    root: String,
    total_nodes: usize,
    total_edges: usize,
    files_indexed: usize,
    nodes: Vec<GraphNodePayload>,
    edges: Vec<GraphEdgePayload>,
    folder_aggregates: Vec<GraphFolderAggregatePayload>,
    projection_limited: bool,
    focus_query: Option<String>,
    focus_node_id: Option<String>,
    focus_matched: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphFocusRequest {
    query: String,
    node_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphSourceSnippetPayload {
    path: String,
    line: usize,
    start_line: usize,
    end_line: usize,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProofArtifactPayload {
    path: String,
    modified_at: String,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInspectionPayload {
    available: bool,
    git_root: Option<String>,
    branch: Option<String>,
    detected_languages: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HubMessageRequest {
    recipient_lane: String,
    body: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HubClaimRequest {
    project_root: String,
    lane_id: String,
    path: String,
    reason: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanFindingPayload {
    rule_id: String,
    severity: String,
    title: String,
    detail: String,
    file: String,
    line: u32,
    snippet: Option<String>,
    doc: Option<String>,
    waiver_id: Option<String>,
    waiver_owner: Option<String>,
    waiver_reason: Option<String>,
    waiver_expires: Option<String>,
    waiver_source: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopFindingWaiverRequest {
    path: String,
    rule_id: String,
    owner: String,
    reason: String,
    expires: Option<String>,
}

#[derive(Deserialize)]
struct PackagedScanScope {
    mode: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopScanTarget {
    id: String,
    label: String,
    description: String,
    mode: String,
    crate_name: Option<String>,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    base: Option<String>,
    #[serde(default)]
    head: Option<String>,
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
}

#[derive(Deserialize)]
struct CargoMetadataPackage {
    name: String,
    manifest_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackagedScanPayload {
    ok: bool,
    violations: Vec<ScanFindingPayload>,
    warnings: Vec<ScanFindingPayload>,
    waived: Vec<ScanFindingPayload>,
    scope: PackagedScanScope,
}

#[tauri::command]
fn desktop_status() -> DesktopStatus {
    DesktopStatus {
        app: "Enforcer",
        shell: "tauri",
        binding_mode: "mixed-live-and-staged",
    }
}

#[tauri::command]
fn load_engine_capabilities() -> EngineCapabilityPayload {
    // Product capability metadata is owned by the Rust control plane. The UI
    // may filter it, but cannot promote a partial or planned surface to live.
    EngineCapabilityPayload {
        capabilities: vec![
            EngineCapability { id: "project-topology", domain: "Project", title: "Connected projects and worktrees", state: "partial", source: "Desktop registrations, Git porcelain discovery, bounded literal-registry project observation, and Store status", controls: "Projects directory, root registration, worktree discovery, and observed stack summary", missing: "Engine-owned project-family registry and repository lifecycle", target: Some(global_project_target("projects")), workpacks: vec!["f02", "f03"] },
            EngineCapability { id: "project-lifecycle", domain: "Project", title: "Project setup and lifecycle", state: "partial", source: "Desktop registration, typed scan-scope and policy settings, memory-index status, proof read model, and legacy CI-posture analysis", controls: "Setup readiness map routes to existing project surfaces and labels their real boundary", missing: "Explicit f02 onboarding/baseline, resolved f03 enforcement tie, c11 install/repair/CI wiring, and persisted lifecycle evidence", target: Some(project_target("setup")), workpacks: vec!["f02", "f03", "c11"] },
            EngineCapability { id: "scan-report", domain: "Project", title: "Scan and report", state: "partial", source: "Packaged Enforcer scan command, typed workspace/package/files/diff scope validation, and persisted desktop report cache", controls: "Workspace, executable Cargo package, validated project-relative files, or verified Git diff scan with finding groups and category/rule/file evidence", missing: "Canonical Rust Report persistence, generic domain targets, and named checks", target: Some(project_target("findings")), workpacks: vec!["f01", "g02"] },
            EngineCapability { id: "project-analysis", domain: "Project", title: "Project analysis", state: "partial", source: "Typed Rust desktop boundary over legacy test-doctrine and UI-coupling reports", controls: "Explicit test-posture and ARCH-1.16 analysis runs for the selected project", missing: "Rust-native analyzers, analysis-run persistence/history, and CI execution envelopes", target: Some(project_target("analysis")), workpacks: vec!["g02", "g05"] },
            EngineCapability { id: "finding-actions", domain: "Project", title: "Finding actions", state: "partial", source: "Packaged exact-path waiver registry, typed Rust g03 waiver command, packaged scan overlay, and exact-path Hub claims", controls: "Eligible findings can create one policy-validated project waiver with accountable owner and reason through the typed g03 module; the scan refreshes after the write, while immutable rules remain non-actionable", missing: "Typed defer/comment actions, FixIntent lifecycle, waiver history, expiry/revocation, and report-row closeout", target: Some(project_target("findings")), workpacks: vec!["a08", "g03", "g04"] },
            EngineCapability { id: "rules", domain: "Project", title: "Rules and skills catalog", state: "partial", source: "Desktop rules/rules.json display catalog, typed project overrides, and a separately observed scanner-language stack", controls: "Numbered rule catalog, policy-covered stack facets, definitions, lock state, and overrides", missing: "A production RuleRegistry shared with the packaged scanner; broader rule applicability, fixture/example payloads, and waiver history", target: Some(project_target("rules")), workpacks: vec!["arc-04", "g08"] },
            EngineCapability { id: "policy", domain: "Project", title: "Policy and native tools", state: "partial", source: "enforcer-config project settings, packaged rule catalog, and separate scanner configuration", controls: "Project-wide rule toggles, severity, scan scope, ignore paths, native ties, and eligible exact-path finding waivers", missing: "Expiry visibility, named exemption history, staged policy impact preview, and shared production RuleRegistry", target: Some(project_target("doctrine")), workpacks: vec!["arc-03", "a08", "g05"] },
            EngineCapability { id: "proof", domain: "Project", title: "Proof ledger", state: "partial", source: "enforcer-proof journal replay and project proof read model", controls: "Run inventory, artifact presence, freshness, and PR-ready claim state", missing: "Proof recording/routing, artifact digest verification, and profile selection", target: Some(project_target("proofs")), workpacks: vec!["arc-17"] },
            EngineCapability { id: "code-graph", domain: "Intelligence", title: "Code graph", state: "partial", source: "X06 Store CodeGraph projection with typed symbols and edges", controls: "Explicit index, symbol facets, bounded pan/zoom topology", missing: "GPU/LOD rendering, neighbour expansion, trace queries, and large-repository readiness", target: Some(project_target("memory")), workpacks: vec!["x06", "g09"] },
            EngineCapability { id: "retrieval", domain: "Intelligence", title: "Retrieval and graph search", state: "partial", source: "Persisted Store and deterministic BM25 graph search", controls: "Typed query results and source-node jump targets", missing: "Semantic fusion, reranking, context-pack explanation, and model-backed synthesis", target: Some(project_target("memory")), workpacks: vec!["x06", "g09"] },
            EngineCapability { id: "learning", domain: "Intelligence", title: "Learning and parity evidence", state: "evidence", source: "X06 proof and parity artifacts", controls: "Read-only proof, model-health, and parity inspection", missing: "Verified t0/t1/t2 learning lifecycle and runtime evidence navigation", target: Some(project_target("memory")), workpacks: vec!["x05", "x06", "g09"] },
            EngineCapability { id: "hub", domain: "Harness", title: "Lane Hub", state: "partial", source: "Typed coordination ledger fold and hash-chained Rust API", controls: "Lanes, inbox, claims, latest task states, workers, messages, acknowledgements, and exact-path claims", missing: "Lane lifecycle, lease projection, and safe code-fix execution dispatch", target: Some(hub_target(Some("lanes"))), workpacks: vec!["arc-16", "g06"] },
            EngineCapability { id: "dispatch", domain: "Harness", title: "Fix dispatch", state: "planned", source: "No typed FixIntent engine record exists yet", controls: "Scan finding inspector exposes the ownership handoff and the unavailable lifecycle after it", missing: "Validated, deduplicated FixIntent, agent pickup, disposition, closeout, token gate, verification, proof, and report-row state", target: Some(project_target("findings")), workpacks: vec!["g04", "g07", "d26"] },
            EngineCapability { id: "harness-adapters", domain: "Harness", title: "Harness adapters and installation", state: "partial", source: "Rust enforcer-install discovery with capability evidence", controls: "Hub -> Adapters shows present or absent homes, source paths, and declared capability evidence", missing: "Adapter verification, hook installation, repair, and desktop onboarding", target: Some(hub_target(Some("harnesses"))), workpacks: vec!["c01", "c03", "c06", "c07", "c11"] },
            EngineCapability { id: "security", domain: "Assurance", title: "Security and money-critical policy", state: "partial", source: "Rust enforcer-security validators and neutral money-critical-security profile", controls: "Selected-project Assurance inventory for profile rules, categories, invariants, and activation state", missing: "Project profile ingestion, runtime security findings, threat/invariant evidence, and CI execution visibility", target: Some(project_target("assurance")), workpacks: vec!["h01", "h02", "h03", "h04", "h05", "h06", "h07", "h08", "h11", "h12"] },
            EngineCapability { id: "harness-runs", domain: "Assurance", title: "Harness run history", state: "partial", source: "enforcer-harness NDJSON run store and redacted query APIs", controls: "Selected-project run history, latest failure, diagnostics, and bounded artifact inspection", missing: "Desktop execution, pin/prune/reset controls, and CI run ingestion", target: Some(project_target("runs")), workpacks: vec!["arc-18", "arc-23"] },
            EngineCapability { id: "self-enforcement", domain: "Assurance", title: "Self-enforcement and CI", state: "partial", source: "Current CLI checks, CI parity module, and workspace validation commands", controls: "Harness run history is visible in Runs; no CI parity or branch-protection control is exposed", missing: "Strict dogfood dashboard, CI parity view, baseline ratchet, and branch protection", target: Some(project_target("runs")), workpacks: vec!["a10", "c10", "d02", "d11", "d28", "x04", "z01"] },
            EngineCapability { id: "planning", domain: "Planning", title: "Plans, rules, and mechanization", state: "partial", source: "Rust read model of authored workpacks, with project Rules kept separate", controls: "Engine Workpacks filters the declared plan index and shows each workpack's ownership, dependencies, plan status, and desktop placement", missing: "Plan scaffold/validation UI, workpack execution truth, and mechanization backlog control", target: Some(global_project_target("engine")), workpacks: vec!["b01", "b02", "b03", "b05", "d01", "d08"] },
        ],
    }
}

#[tauri::command]
fn load_desktop_rule_catalog() -> Result<DesktopRuleCatalog, String> {
    let path = resolve_pack_root()?.join("rules").join("rules.json");
    let bytes = std::fs::read(&path)
        .map_err(|error| format!("cannot read desktop rule catalog {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot decode desktop rule catalog {}: {error}", path.display()))
}

#[tauri::command]
fn load_project_rule_coverage(root: String) -> Result<ProjectRuleCoveragePayload, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let catalog = load_desktop_rule_catalog()?;
    let detected_languages = detect_project_languages(&root_path);
    let project_paths = walk_repo_files(&root_path)?
        .into_iter()
        .filter_map(|path| path.strip_prefix(&root_path).ok().map(normalize_project_path))
        .collect::<Vec<_>>();
    let catalog_languages = catalog
        .languages
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let observed_without_catalog = detected_languages
        .iter()
        .filter(|language| language.as_str() != "common" && !catalog_languages.contains(*language))
        .cloned()
        .collect();
    let settings = enforcer_ui::settings::read::load_settings_view(&root_path.join("enforce.config.json")).ok();
    let overrides = settings
        .as_ref()
        .map(|view| view.rule_toggles.iter().map(|row| (row.rule_id.as_str(), row)).collect::<BTreeMap<_, _>>())
        .unwrap_or_default();
    let rules = catalog
        .rules
        .into_iter()
        .map(|rule| {
            let override_row = overrides.get(rule.id.as_str()).copied();
            let scope = if rule.language == "common" {
                "universal"
            } else if detected_languages.contains(&rule.language) {
                "language-match"
            } else {
                "not-detected"
            };
            let effective_severity = override_row
                .and_then(|row| row.severity.clone())
                .unwrap_or_else(|| rule.severity.clone());
            let state = match override_row {
                Some(row) if !row.enabled => "disabled",
                Some(row) if row.severity.is_some() => "severity-override",
                Some(_) => "explicit-enabled",
                None => "registry-default",
            };
            let path_match = evaluate_rule_path_match(&rule.applies_to, &project_paths);
            ProjectRuleCoverageRow {
                rule_id: rule.id,
                language: rule.language,
                scope: scope.to_owned(),
                effective_severity,
                state: state.to_owned(),
                path_match_status: path_match.status,
                matched_path_count: path_match.matched_path_count,
            }
        })
        .collect();
    Ok(ProjectRuleCoveragePayload {
        detected_languages,
        catalog_languages: catalog_languages.into_iter().collect(),
        observed_without_catalog,
        settings_status: if settings.is_some() { "loaded" } else { "unavailable" }.to_owned(),
        rules,
    })
}

struct RulePathMatch {
    status: &'static str,
    matched_path_count: usize,
}

fn normalize_project_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn evaluate_rule_path_match(patterns: &[String], project_paths: &[String]) -> RulePathMatch {
    if patterns.is_empty() {
        return RulePathMatch {
            status: "unscoped",
            matched_path_count: 0,
        };
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = match GlobBuilder::new(pattern)
            .literal_separator(true)
            .backslash_escape(false)
            .build()
        {
            Ok(glob) => glob,
            Err(_) => {
                return RulePathMatch {
                    status: "invalid-pattern",
                    matched_path_count: 0,
                }
            }
        };
        builder.add(glob);
    }
    let set = match builder.build() {
        Ok(set) => set,
        Err(_) => {
            return RulePathMatch {
                status: "invalid-pattern",
                matched_path_count: 0,
            }
        }
    };
    let matched_path_count = project_paths.iter().filter(|path| set.is_match(path)).count();
    RulePathMatch {
        status: if matched_path_count == 0 { "no-match" } else { "matched" },
        matched_path_count,
    }
}

#[tauri::command]
fn load_harness_discovery() -> Result<HarnessDiscoveryPayload, String> {
    let env = enforcer_install::detect::RealEnv;
    let fs = enforcer_install::detect::RealFs;
    let harnesses = enforcer_install::detect::detect_harnesses(&env, &fs)
        .map_err(|error| format!("harness discovery failed: {error}"))?;
    Ok(HarnessDiscoveryPayload {
        harnesses,
        runtime: "enforcer-install::detect::detect_harnesses",
        verification: "Discovery only. Adapter verification, hook installation, and repair are not wired into desktop.",
    })
}

#[tauri::command]
fn load_workpack_index() -> Result<WorkpackIndexPayload, String> {
    let source_path = resolve_pack_root()?
        .join("docs")
        .join("plans")
        .join("enforcer-selfhost-plan")
        .join("WORKPACK_INDEX.md");
    let source = std::fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "cannot read workpack index at {}: {error}",
            source_path.display()
        )
    })?;
    let rows = parse_workpack_index(&source);
    if rows.is_empty() {
        return Err(format!(
            "workpack index at {} did not contain recognized status rows",
            source_path.display()
        ));
    }
    let mut status_counts = BTreeMap::new();
    for row in &rows {
        *status_counts.entry(row.status.clone()).or_insert(0) += 1;
    }
    Ok(WorkpackIndexPayload {
        source_path: source_path.display().to_string(),
        rows,
        status_counts,
        caveat: "Declared Markdown index status only. It is not execution, proof, or repository-completion truth.",
    })
}

#[tauri::command]
fn load_security_profile(root: String) -> Result<SecurityProfilePayload, String> {
    let source_path = resolve_pack_root()?
        .join("profiles")
        .join("money-critical-security.json");
    let source = std::fs::read(&source_path).map_err(|error| {
        format!(
            "cannot read security profile at {}: {error}",
            source_path.display()
        )
    })?;
    let profile: SecurityProfileFile = serde_json::from_slice(&source).map_err(|error| {
        format!(
            "cannot decode security profile at {}: {error}",
            source_path.display()
        )
    })?;
    let project_root = PathBuf::from(root);
    let activation = enforcer_security::activation::load_project_activation(&project_root)?;
    let project_activation = match &activation {
        Some(activation) => format!(
            "Activated by {} for {}. Scan coverage and CI gating are not implemented yet.",
            activation.owner, activation.source_spec
        ),
        None => "Not activated. Supply a source specification, owner, and reason to record activation intent; coverage and CI gating remain unavailable.".to_owned(),
    };
    Ok(SecurityProfilePayload {
        source_path: source_path.display().to_string(),
        profile_name: profile.profile_name,
        required_test_categories: profile.required_test_categories,
        invariants: profile.invariants,
        rules: profile.rules,
        activated: activation.is_some(),
        project_activation,
        caveat: "This profile is available and its rules are backed. Activation records intent only; they do not mean the selected project is covered, scanned, or CI-gated.",
    })
}

#[tauri::command]
fn activate_security_profile(
    root: String,
    request: SecurityActivationRequest,
) -> Result<SecurityProfilePayload, String> {
    enforcer_security::activation::write_project_activation(
        &PathBuf::from(&root),
        &enforcer_security::activation::SecurityProfileActivation {
            schema_version: 1,
            profile_name: enforcer_security::activation::MONEY_CRITICAL_PROFILE.to_owned(),
            source_spec: request.source_spec,
            owner: request.owner,
            reason: request.reason,
        },
    )?;
    load_security_profile(root)
}

fn parse_workpack_index(source: &str) -> Vec<WorkpackIndexRow> {
    source
        .lines()
        .filter_map(parse_workpack_index_row)
        .collect()
}

fn parse_workpack_index_row(line: &str) -> Option<WorkpackIndexRow> {
    let cells = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>();
    if cells.len() != 8 || matches!(cells[0], "Status" | "--------") {
        return None;
    }
    let workpack = cells[1];
    let link_start = workpack.find('[')?;
    let link_end = workpack.find("](")?;
    let path_end = workpack[link_end + 2..].find(')')? + link_end + 2;
    let label = workpack[link_start + 1..link_end].trim();
    let (id, title) = label.split_once(' ').unwrap_or((label, label));
    Some(WorkpackIndexRow {
        id: id.to_owned(),
        title: title.to_owned(),
        status: cells[0].to_owned(),
        track: cells[2].to_owned(),
        owns: cells[3].to_owned(),
        tier: cells[5].to_owned(),
        dependencies: cells[6].to_owned(),
        parallel_safe_with: cells[7].to_owned(),
        source_path: workpack[link_end + 2..path_end]
            .trim_start_matches("./")
            .to_owned(),
    })
}

#[tauri::command]
async fn run_packaged_scan(
    root: String,
    target: Option<DesktopScanTarget>,
) -> Result<DesktopReportPayload, String> {
    tauri::async_runtime::spawn_blocking(move || run_packaged_scan_sync(root, target))
        .await
        .map_err(|error| format!("scan task failed: {error}"))?
}

#[tauri::command]
async fn waive_packaged_finding(
    root: String,
    request: DesktopFindingWaiverRequest,
) -> Result<DesktopReportPayload, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root_path = PathBuf::from(&root);
        if !root_path.is_dir() {
            return Err(format!("project root is not a directory: {root}"));
        }
        write_desktop_file_rule_waiver(&root_path, request)?;
        run_packaged_scan_sync(root, None)
    })
    .await
    .map_err(|error| format!("waiver task failed: {error}"))?
}

/// Why a typed desktop command step (waiver persistence, packaged resource
/// resolution) rejected its input or failed.
// BRAND-INVARIANT: wraps exactly one already-rendered, human-readable
// failure message; constructed only by the desktop command paths below and
// unwrapped only through the `From<DesktopCommandError> for String` boundary.
#[derive(Debug)]
struct DesktopCommandError(String);

impl From<DesktopCommandError> for String {
    fn from(error: DesktopCommandError) -> Self {
        error.0
    }
}

/// Persist one exact-path finding waiver through the typed g03 Rust module
/// ([`enforcer_ui::actions::file_rule_waiver`]) instead of shelling to the
/// Node packaged-waiver bridge. The rule identifier is validated against the
/// packaged catalog's *waivable* rules, so both an unknown rule and a
/// non-waivable (immutable) rule are rejected before anything is written —
/// which also keeps the packaged scanner able to reload the registry.
fn write_desktop_file_rule_waiver(
    root_path: &Path,
    request: DesktopFindingWaiverRequest,
) -> Result<(), DesktopCommandError> {
    let rule_id = RuleId::from_str(request.rule_id.trim().to_ascii_uppercase().as_str())
        .map_err(|error| {
            DesktopCommandError(format!(
                "waiver references an invalid rule id `{}`: {error}",
                request.rule_id
            ))
        })?;
    let expires = match request.expires.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => Some(WaiverDate::from_str(value).map_err(
            |error| DesktopCommandError(format!("invalid waiver expiry `{value}`: {error}")),
        )?),
        _ => None,
    };
    let waivable_rules = desktop_waivable_rule_registry()?;
    let typed_request = FileRuleWaiverRequest {
        path: request.path,
        rule_id,
        owner: request.owner,
        reason: request.reason,
        expires,
    };
    upsert_file_rule_waiver(root_path, &waivable_rules, current_waiver_date()?, &typed_request)
        .map(|_| ())
        .map_err(|error| DesktopCommandError(format!("waiver rejected: {error}")))
}

/// Build the rule registry the desktop waiver path validates against, from the
/// packaged display catalog's waivable rules only.
///
/// The display catalog is not the canonical fixture-linked rule registry, so
/// the validator/fixture/tier linkage below is a transient, in-memory
/// membership gate — it is never surfaced or persisted. Its sole job is to let
/// [`upsert_file_rule_waiver`] accept a waiver iff the rule is a known,
/// waivable packaged rule (parity with the retired Node bridge).
fn desktop_waivable_rule_registry() -> Result<RuleRegistry, DesktopCommandError> {
    let catalog = load_desktop_rule_catalog().map_err(DesktopCommandError)?;
    let records = catalog
        .rules
        .into_iter()
        .filter(|rule| rule.waivable)
        .map(|rule| {
            let DesktopRuleCatalogRule {
                id,
                title,
                validator,
                doc,
                ..
            } = rule;
            let rule_id = RuleId::from_str(&id).map_err(|error| {
                DesktopCommandError(format!("packaged rule id `{id}` is invalid: {error}"))
            })?;
            let validator_path = if validator.trim().is_empty() {
                format!("packaged-catalog://{id}")
            } else {
                validator
            };
            let doc_anchor = if doc.trim().is_empty() {
                format!("packaged-catalog://{id}")
            } else {
                doc
            };
            Ok(RuleRecord {
                rule_id,
                version: 1,
                title,
                tier: Tier::T1,
                validator: ValidatorRef {
                    // ALLOC-JUSTIFICATION: ValidatorRef owns its crate label;
                    // one small allocation per waivable catalog rule.
                    crate_name: "ocentra-enforcer-packaged".to_owned(),
                    path: validator_path,
                },
                fixtures: FixtureRef {
                    fail: format!("packaged-catalog://{id}/fail"),
                    pass: format!("packaged-catalog://{id}/pass"),
                },
                doc_anchor,
                tags: Vec::new(),
                params: serde_json::json!(null),
            })
        })
        .collect::<Result<Vec<_>, DesktopCommandError>>()?;
    RuleRegistry::from_records(records).map_err(|error| {
        DesktopCommandError(format!(
            "cannot build packaged waivable-rule registry: {error}"
        ))
    })
}

/// Today's calendar date (UTC) as a strict [`WaiverDate`], for expiry checks.
/// The day-count split uses Howard Hinnant's `civil_from_days` algorithm.
fn current_waiver_date() -> Result<WaiverDate, DesktopCommandError> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            DesktopCommandError(format!("system clock is before the Unix epoch: {error}"))
        })?
        .as_secs();
    // CAST-JUSTIFICATION: the epoch-day count is far below i64::MAX and the
    // civil-date algorithm below is defined over signed day arithmetic.
    let z = (seconds / 86_400) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    // CAST-JUSTIFICATION: the algorithm guarantees 1 <= day <= 31 and
    // 1 <= month <= 12, so both values fit u8 exactly.
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = yoe + era * 400 + i64::from(month <= 2);
    let year = u16::try_from(year).map_err(|_| {
        DesktopCommandError(format!("current year {year} is out of range for waiver dates"))
    })?;
    WaiverDate::new(year, month, day).map_err(|error| {
        DesktopCommandError(format!("cannot build today's waiver date: {error}"))
    })
}

#[tauri::command]
async fn load_scan_targets(root: String) -> Result<Vec<DesktopScanTarget>, String> {
    tauri::async_runtime::spawn_blocking(move || discover_scan_targets(Path::new(&root)))
        .await
        .map_err(|error| format!("scan target discovery failed: {error}"))?
}

fn workspace_scan_target() -> DesktopScanTarget {
    DesktopScanTarget {
        id: "workspace".to_owned(),
        label: "Entire workspace".to_owned(),
        description: "Run the packaged scanner across the selected project root.".to_owned(),
        mode: "workspace".to_owned(),
        crate_name: None,
        files: Vec::new(),
        base: None,
        head: None,
    }
}

fn discover_scan_targets(root: &Path) -> Result<Vec<DesktopScanTarget>, String> {
    if !root.is_dir() {
        return Err(format!(
            "project root is not a directory: {}",
            root.display()
        ));
    }

    let mut targets = vec![workspace_scan_target()];
    targets.extend(discover_project_path_targets(root)?);
    let manifest = root.join("Cargo.toml");
    if !manifest.is_file() {
        return Ok(targets);
    }

    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(&manifest)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot discover Cargo packages: {error}"))?;
    if !output.status.success() {
        return Ok(targets);
    }
    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot decode Cargo package metadata: {error}"))?;
    let mut packages = metadata.packages;
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    for package in packages {
        let manifest_path = PathBuf::from(&package.manifest_path);
        let relative = manifest_path
            .parent()
            .and_then(|path| path.strip_prefix(root).ok())
            .map(|path| path.display().to_string())
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| ".".to_owned());
        targets.push(DesktopScanTarget {
            id: format!("crate:{}", package.name),
            label: package.name.clone(),
            description: format!(
                "Rust package at {relative}; passes --crate {} to the packaged scanner.",
                package.name
            ),
            mode: "crate".to_owned(),
            crate_name: Some(package.name),
            files: Vec::new(),
            base: None,
            head: None,
        });
    }
    Ok(targets)
}

fn discover_project_path_targets(root: &Path) -> Result<Vec<DesktopScanTarget>, String> {
    const IGNORED: &[&str] = &[
        ".enforce",
        ".git",
        ".ledger",
        "coverage",
        "dist",
        "node_modules",
        "target",
    ];
    let mut directories = std::fs::read_dir(root)
        .map_err(|error| format!("cannot inspect project directories: {error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|file_type| file_type.is_dir() && !file_type.is_symlink())
                .and_then(|_| entry.file_name().into_string().ok())
        })
        .filter(|name| !name.starts_with('.') && !IGNORED.contains(&name.as_str()))
        .collect::<Vec<_>>();
    directories.sort();
    directories.truncate(16);
    Ok(directories
        .into_iter()
        .map(|directory| DesktopScanTarget {
            id: format!("paths:{directory}"),
            label: directory.clone(),
            description: format!(
                "Project directory {directory}; passes it as a validated --files path to the packaged scanner."
            ),
            mode: "files".to_owned(),
            crate_name: None,
            files: vec![directory],
            base: None,
            head: None,
        })
        .collect())
}

fn validated_scan_target(
    root: &Path,
    target: Option<DesktopScanTarget>,
) -> Result<DesktopScanTarget, String> {
    let requested = target.unwrap_or_else(workspace_scan_target);
    if requested.mode == "files" {
        let files = requested
            .files
            .iter()
            .map(|file| validate_desktop_scan_path(root, file))
            .collect::<Result<Vec<_>, _>>()?;
        if files.is_empty() {
            return Err("file scan target requires at least one project-relative path".to_owned());
        }
        return Ok(DesktopScanTarget {
            id: format!("files:{}", files.join(",")),
            label: if files.len() == 1 {
                files[0].clone()
            } else {
                format!("{} selected paths", files.len())
            },
            description: "Explicit project-relative file or directory scan.".to_owned(),
            mode: "files".to_owned(),
            crate_name: None,
            files,
            base: None,
            head: None,
        });
    }
    if requested.mode == "diff" {
        let base = validate_desktop_git_revision(root, requested.base.as_deref(), "base")?;
        let head = validate_desktop_git_revision(root, requested.head.as_deref(), "head")?;
        return Ok(DesktopScanTarget {
            id: format!("diff:{base}..{head}"),
            label: format!("{base}..{head}"),
            description: "Files changed between two verified Git revisions.".to_owned(),
            mode: "diff".to_owned(),
            crate_name: None,
            files: Vec::new(),
            base: Some(base),
            head: Some(head),
        });
    }
    discover_scan_targets(root)?
        .into_iter()
        .find(|candidate| {
            candidate.id == requested.id
                && candidate.mode == requested.mode
                && candidate.crate_name == requested.crate_name
        })
        .ok_or_else(|| format!("unsupported desktop scan target: {}", requested.id))
}

fn validate_desktop_scan_path(root: &Path, raw_path: &str) -> Result<String, String> {
    let relative = raw_path.trim().replace('\\', "/");
    if relative.is_empty()
        || Path::new(&relative).is_absolute()
        || relative.contains(':')
        || relative
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(format!(
            "scan path must be a non-empty project-relative file or directory: {raw_path}"
        ));
    }
    let candidate = root.join(&relative);
    if !candidate.exists() {
        return Err(format!(
            "scan path does not exist under the selected project: {relative}"
        ));
    }
    Ok(relative)
}

fn validate_desktop_git_revision(
    root: &Path,
    revision: Option<&str>,
    label: &str,
) -> Result<String, String> {
    let revision = revision
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("diff scan target requires a {label} revision"))?;
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg(format!("{revision}^{{commit}}"))
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot validate Git {label} revision: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git {label} revision does not resolve to a commit: {revision}"
        ));
    }
    Ok(revision.to_owned())
}

fn run_packaged_scan_sync(
    root: String,
    target: Option<DesktopScanTarget>,
) -> Result<DesktopReportPayload, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let target = validated_scan_target(&root_path, target)?;
    let pack_root = resolve_pack_root()?;
    let script = pack_root.join("scripts").join("ocentra-enforcer.mjs");
    if !script.is_file() {
        return Err(format!(
            "packaged Enforcer scanner is unavailable at {}; Rust Report persistence has not landed for this desktop build",
            script.display()
        ));
    }
    let mut command = Command::new("node");
    command.arg(&script).arg("scan").arg("--root").arg(&root);
    if target.mode == "crate" {
        command.arg("--crate").arg(
            target
                .crate_name
                .as_deref()
                .expect("validated crate target must include a package name"),
        );
    } else if target.mode == "files" {
        command.arg("--files").args(&target.files);
    } else if target.mode == "diff" {
        command
            .arg("--base")
            .arg(
                target
                    .base
                    .as_deref()
                    .expect("validated diff target must include base"),
            )
            .arg("--head")
            .arg(
                target
                    .head
                    .as_deref()
                    .expect("validated diff target must include head"),
            );
    } else {
        command.arg("--workspace");
    }
    let output = command
        .arg("--json")
        .current_dir(&pack_root)
        .output()
        .map_err(|error| format!("cannot start packaged Enforcer scanner: {error}"))?;
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("packaged Enforcer scanner produced non-UTF8 JSON: {error}"))?;
    let scan: PackagedScanPayload = serde_json::from_str(&stdout).map_err(|error| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        format!("cannot decode packaged Enforcer scan report: {error}; stderr: {stderr}")
    })?;
    let total_count = scan.violations.len() + scan.warnings.len() + scan.waived.len();
    let report = DesktopReportPayload {
        ok: scan.ok,
        scope: scan.scope.mode,
        violations: scan.violations,
        warnings: scan.warnings,
        waived: scan.waived,
        total_count,
        runtime: "packaged-enforcer-command".to_owned(),
        persistence: "desktop-cached-packaged-report".to_owned(),
        generated_at: store_timestamp(),
        run_id: desktop_scan_run_id(),
        target_label: target.label,
    };
    persist_desktop_report(&root_path, &report)?;
    Ok(report)
}

#[tauri::command]
async fn run_legacy_analysis(
    root: String,
    kind: LegacyAnalysisKind,
) -> Result<LegacyAnalysisRunPayload, String> {
    tauri::async_runtime::spawn_blocking(move || run_legacy_analysis_sync(root, kind))
        .await
        .map_err(|error| format!("analysis task failed: {error}"))?
}

fn run_legacy_analysis_sync(
    root: String,
    kind: LegacyAnalysisKind,
) -> Result<LegacyAnalysisRunPayload, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let pack_root = resolve_pack_root()?;
    let script = pack_root.join("scripts").join("desktop-analysis.mjs");
    if !script.is_file() {
        return Err(format!(
            "legacy analysis bridge is unavailable at {}; the desktop package is incomplete",
            script.display()
        ));
    }
    let kind_name = match kind {
        LegacyAnalysisKind::TestDoctrine => "test-doctrine",
        LegacyAnalysisKind::UiLogicCoupling => "ui-logic-coupling",
    };
    let output = Command::new("node")
        .arg(&script)
        .arg("--root")
        .arg(&root)
        .arg("--kind")
        .arg(kind_name)
        .current_dir(&pack_root)
        .output()
        .map_err(|error| format!("cannot start legacy analysis bridge: {error}"))?;
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("legacy analysis bridge produced non-UTF8 JSON: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "legacy analysis bridge failed with {}; stderr: {stderr}",
            output.status
        ));
    }
    let envelope: LegacyAnalysisEnvelope = serde_json::from_str(&stdout).map_err(|error| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        format!("cannot decode legacy analysis response: {error}; stderr: {stderr}")
    })?;
    if envelope.schema_version != 1 {
        return Err(format!(
            "unsupported legacy analysis schema version {}; expected 1",
            envelope.schema_version
        ));
    }
    if envelope.analysis_kind != kind {
        return Err("legacy analysis bridge returned a different analysis kind".to_owned());
    }
    let metadata = AnalysisRunMetadata {
        schema_version: 1,
        runtime: "legacy-node-analysis-bridge",
        source: "scripts/desktop-analysis.mjs",
        state: "partial",
        generated_at: store_timestamp(),
        caveat: "Legacy Node analysis with a typed Rust desktop boundary. Rust-native analysis persistence, history, and CI execution visibility are not implemented.",
    };
    match kind {
        LegacyAnalysisKind::TestDoctrine => {
            let report = serde_json::from_value(envelope.report)
                .map_err(|error| format!("cannot decode typed test-doctrine report: {error}"))?;
            Ok(LegacyAnalysisRunPayload::TestDoctrine { metadata, report })
        }
        LegacyAnalysisKind::UiLogicCoupling => {
            let report = serde_json::from_value(envelope.report)
                .map_err(|error| format!("cannot decode typed UI coupling report: {error}"))?;
            Ok(LegacyAnalysisRunPayload::UiLogicCoupling { metadata, report })
        }
    }
}

#[tauri::command]
fn load_harness_runs(root: String) -> Result<HarnessRunsPayload, String> {
    load_harness_runs_from(&PathBuf::from(root))
}

fn load_harness_runs_from(root: &Path) -> Result<HarnessRunsPayload, String> {
    if !root.is_dir() {
        return Err(format!(
            "project root is not a directory: {}",
            root.display()
        ));
    }
    let config = enforcer_harness::config::HarnessConfig::default();
    let query = enforcer_harness::query::RunQuery {
        limit: Some(100),
        ..enforcer_harness::query::RunQuery::default()
    };
    let runs = enforcer_harness::query::list_runs(root, &config, &query)
        .map_err(|error| format!("cannot read typed harness run store: {error}"))?
        .iter()
        .map(harness_run_row)
        .collect();
    let (found_failure, failed_run, diagnostics) =
        enforcer_harness::query::last_failure(root, &config, &query, Some(25))
            .map_err(|error| format!("cannot read latest harness failure: {error}"))?;
    let last_failure = if found_failure {
        failed_run.map(|run| HarnessFailurePayload {
            run: harness_run_row(&run),
            diagnostics: diagnostics.iter().map(harness_diagnostic).collect(),
        })
    } else {
        None
    };
    Ok(HarnessRunsPayload {
        root: root.display().to_string(),
        storage: "enforcer-harness query across .enforce and legacy .ocentra-enforcer storage roots",
        runs,
        last_failure,
        caveat: "Read-only typed run history. The desktop cannot execute, pin, prune, reset, or repair harness runs yet.",
    })
}

#[tauri::command]
fn load_harness_run_detail(
    root: String,
    run_id: String,
) -> Result<HarnessRunDetailPayload, String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err(format!(
            "project root is not a directory: {}",
            root.display()
        ));
    }
    load_harness_run_detail_from(&root, &run_id)
}

fn load_harness_run_detail_from(
    root: &Path,
    run_id: &str,
) -> Result<HarnessRunDetailPayload, String> {
    let config = enforcer_harness::config::HarnessConfig::default();
    let query = enforcer_harness::query::RunQuery {
        run_id: Some(run_id.to_owned()),
        ..enforcer_harness::query::RunQuery::default()
    };
    let run = enforcer_harness::query::run_summary(root, &config, &query)
        .map_err(|error| format!("cannot read typed harness run summary: {error}"))?
        .ok_or_else(|| format!("harness run is not available: {run_id}"))?;
    let (_, _, diagnostics) = enforcer_harness::query::run_diagnostics(
        root,
        &config,
        &query,
        enforcer_harness::query::DiagnosticsFilter {
            limit: Some(100),
            ..enforcer_harness::query::DiagnosticsFilter::default()
        },
    )
    .map_err(|error| format!("cannot read typed harness diagnostics: {error}"))?;
    let stdout = load_harness_artifact(root, &config, &query, "stdout")?;
    let stderr = load_harness_artifact(root, &config, &query, "stderr")?;
    Ok(HarnessRunDetailPayload {
        run: harness_run_row(&run),
        diagnostics: diagnostics.iter().map(harness_diagnostic).collect(),
        stdout,
        stderr,
        caveat: "Artifacts are read through enforcer-harness, redacted by its shared redactor, and bounded by the harness artifact limit.",
    })
}

fn load_harness_artifact(
    root: &Path,
    config: &enforcer_harness::config::HarnessConfig,
    query: &enforcer_harness::query::RunQuery,
    artifact: &str,
) -> Result<HarnessArtifactPayload, String> {
    let (available, _, content, error) =
        enforcer_harness::query::read_artifact(root, config, query, artifact, Some(8_000))
            .map_err(|source| format!("cannot read harness {artifact} artifact: {source}"))?;
    Ok(HarnessArtifactPayload {
        available,
        content,
        error,
    })
}

fn harness_run_row(value: &serde_json::Value) -> HarnessRunRow {
    let string = |field| {
        value
            .get(field)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    HarnessRunRow {
        run_id: string("runId"),
        tool: string("tool"),
        language: value
            .get("language")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        command: value
            .get("command")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        status: string("status"),
        exit_code: value
            .get("exitCode")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_default(),
        started_at: string("startedAt"),
        ended_at: string("endedAt"),
        diagnostic_count: value
            .get("diagnosticCount")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default() as usize,
        pinned: value
            .get("pinned")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        storage_root: value
            .get("storage")
            .and_then(|storage| storage.get("root"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    }
}

fn harness_diagnostic(value: &serde_json::Value) -> HarnessDiagnosticPayload {
    let string = |field| {
        value
            .get(field)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    HarnessDiagnosticPayload {
        severity: string("severity"),
        rule_id: string("ruleId"),
        file: string("file"),
        line: value
            .get("line")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default() as usize,
        message: string("message"),
        source: value
            .get("source")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }
}

/// Environment variable that lets an operator point the desktop shell at an
/// explicit packaged-resources directory.
const PACK_ROOT_ENV: &str = "ENFORCER_PACK_ROOT";

/// Marker-validated packaged Enforcer resource root.
// BRAND-INVARIANT: constructed only after the directory proves it carries
// the packaged markers (rules/rules.json and scripts/ocentra-enforcer.mjs),
// so every consumer can join pack-relative resource paths safely.
struct PackRoot(PathBuf);

impl std::ops::Deref for PackRoot {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for PackRoot {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// Resolve the packaged Enforcer resource root at runtime, in priority order,
/// with NO silent fallback to a compile-time developer path:
///
/// 1. the `ENFORCER_PACK_ROOT` environment variable (explicit override);
/// 2. a marker-validated ancestor of the running executable (covers both the
///    installed layout and the development tree).
///
/// A directory qualifies only when it carries the two load-bearing resources
/// every read depends on: the display rule catalog and the packaged scanner
/// entry point. If nothing resolves, the desktop surfaces the returned error
/// instead of reading rules, scripts, profiles, or the plan index from a
/// stale developer path baked in at build time.
fn resolve_pack_root() -> Result<PackRoot, DesktopCommandError> {
    let has_markers = |candidate: &Path| {
        candidate.join("rules").join("rules.json").is_file()
            && candidate
                .join("scripts")
                .join("ocentra-enforcer.mjs")
                .is_file()
    };

    if let Some(raw) = std::env::var_os(PACK_ROOT_ENV) {
        let candidate = PathBuf::from(&raw);
        if has_markers(&candidate) {
            return Ok(PackRoot(candidate));
        }
        return Err(DesktopCommandError(format!(
            "{PACK_ROOT_ENV} is set to `{}`, but that directory is missing the packaged Enforcer resources (rules/rules.json and scripts/ocentra-enforcer.mjs)",
            candidate.display()
        )));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(found) = exe.ancestors().find(|dir| has_markers(dir)) {
            return Ok(PackRoot(found.to_path_buf()));
        }
    }

    Err(DesktopCommandError(format!(
        "cannot locate the packaged Enforcer resources; set {PACK_ROOT_ENV} to the directory that contains rules/, scripts/, profiles/, and docs/ (checked {PACK_ROOT_ENV} and every ancestor of the executable)"
    )))
}

/// Test-only infallible accessor: the source tree the tests run from always
/// carries the pack markers, so resolution must succeed. Production code paths
/// use [`resolve_pack_root`] and surface the error state instead.
#[cfg(test)]
fn desktop_workspace_root() -> PackRoot {
    resolve_pack_root().expect("packaged resources must resolve from the test build tree")
}

#[tauri::command]
fn load_hub(ledger_root: Option<String>) -> Result<serde_json::Value, String> {
    let root = resolve_hub_ledger_root(ledger_root)?;
    let payload = enforcer_ui::hub::render_hub_from_root(enforcer_ui::hub::RunMode::Human, &root);
    serde_json::to_value(payload)
        .map_err(|error| format!("cannot encode coordination ledger view: {error}"))
}

#[tauri::command]
fn send_hub_message(
    request: HubMessageRequest,
    ledger_root: Option<String>,
) -> Result<serde_json::Value, String> {
    let root = resolve_hub_ledger_root(ledger_root)?;
    let hub = enforcer_coordination::api::open(&root)
        .map_err(|error| format!("cannot open existing coordination identity: {error}"))?;
    let caller = desktop_hub_caller();
    enforcer_coordination::api::send_message(
        &hub,
        &hub.config.default_lane,
        &request.recipient_lane,
        &request.body,
        &caller,
    )
    .map_err(|error| format!("cannot dispatch coordination message: {error}"))?;
    let payload = enforcer_ui::hub::render_hub_from_root(enforcer_ui::hub::RunMode::Human, &root);
    serde_json::to_value(payload)
        .map_err(|error| format!("cannot encode updated coordination ledger view: {error}"))
}

#[tauri::command]
fn acknowledge_hub_message(
    message_id: String,
    ledger_root: Option<String>,
) -> Result<serde_json::Value, String> {
    let root = resolve_hub_ledger_root(ledger_root)?;
    let hub = enforcer_coordination::api::open(&root)
        .map_err(|error| format!("cannot open existing coordination identity: {error}"))?;
    let caller = desktop_hub_caller();
    enforcer_coordination::api::acknowledge_message(
        &hub,
        &hub.config.default_lane,
        &message_id,
        &caller,
    )
    .map_err(|error| format!("cannot acknowledge coordination message: {error}"))?;
    let payload = enforcer_ui::hub::render_hub_from_root(enforcer_ui::hub::RunMode::Human, &root);
    serde_json::to_value(payload)
        .map_err(|error| format!("cannot encode updated coordination ledger view: {error}"))
}

#[tauri::command]
fn create_hub_claim(
    request: HubClaimRequest,
    ledger_root: Option<String>,
) -> Result<serde_json::Value, String> {
    let project_root = PathBuf::from(&request.project_root);
    if !project_root.is_dir() {
        return Err(format!(
            "claim project root is not a directory: {}",
            project_root.display()
        ));
    }
    let path = Path::new(request.path.trim());
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("claim path must be a non-empty project-relative path".to_owned());
    }
    let reason = request.reason.trim();
    if reason.is_empty() {
        return Err("claim reason is required".to_owned());
    }

    let root = resolve_hub_ledger_root(ledger_root)?;
    let hub = enforcer_coordination::api::open(&root)
        .map_err(|error| format!("cannot open existing coordination identity: {error}"))?;
    let lane = request
        .lane_id
        .trim()
        .parse()
        .map_err(|error| format!("invalid claim lane: {error}"))?;
    let owns = vec![path.to_string_lossy().replace('\\', "/")];
    let caller = desktop_project_caller(&project_root);
    let outcome = enforcer_coordination::api::claim_all(
        &hub,
        enforcer_coordination::api::ClaimRequestArgs {
            repo_root: &project_root,
            lane: &lane,
            owns: &owns,
            caller: &caller,
            reason: Some(reason),
        },
    )
    .map_err(|error| format!("cannot create coordination claim: {error}"))?;
    if !outcome.ok {
        return Err(format!(
            "claim blocked by {} existing ownership record(s)",
            outcome.blockers.len()
        ));
    }
    let payload = enforcer_ui::hub::render_hub_from_root(enforcer_ui::hub::RunMode::Human, &root);
    serde_json::to_value(payload)
        .map_err(|error| format!("cannot encode updated coordination ledger view: {error}"))
}

fn resolve_hub_ledger_root(ledger_root: Option<String>) -> Result<PathBuf, String> {
    let root = ledger_root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("E:/ocentra-enforcer/.ledger"));
    if !root.is_dir() {
        return Err(format!(
            "coordination ledger root is not a directory: {}",
            root.display()
        ));
    }
    Ok(root)
}

fn desktop_hub_caller() -> enforcer_coordination::api::CallerContext {
    // The hub caller identity is a label, not a resource read: when the pack
    // root cannot be resolved, degrade to a neutral desktop caller rather than
    // depend on a compile-time path.
    match resolve_pack_root() {
        Ok(root) => desktop_project_caller(&root),
        // ALLOC-JUSTIFICATION: CallerContext owns its identity strings; the
        // three small labels below allocate once per degraded hub call.
        Err(_) => enforcer_coordination::api::CallerContext {
            project_id: "enforcer-desktop".to_owned(),
            // ALLOC-JUSTIFICATION: covered by the CallerContext note above.
            worktree_root: "unavailable".to_owned(),
            branch: "unavailable".to_owned(),
            commit: None,
            codex_thread_id: None,
            codex_session_id: None,
        },
    }
}

fn desktop_project_caller(root: &Path) -> enforcer_coordination::api::CallerContext {
    let project_id = root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("enforcer-desktop")
        .to_owned();
    enforcer_coordination::api::CallerContext {
        project_id,
        worktree_root: root.display().to_string(),
        branch: git_value(&root, &["branch", "--show-current"])
            .unwrap_or_else(|| "unavailable".to_owned()),
        commit: git_value(&root, &["rev-parse", "HEAD"]),
        codex_thread_id: None,
        codex_session_id: None,
    }
}

#[tauri::command]
fn inspect_project(root: String) -> Result<ProjectInspectionPayload, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Ok(ProjectInspectionPayload {
            available: false,
            git_root: None,
            branch: None,
            detected_languages: Vec::new(),
        });
    }
    let detected_languages = detect_project_languages(&root_path);
    Ok(ProjectInspectionPayload {
        available: true,
        git_root: git_value(&root_path, &["rev-parse", "--show-toplevel"]),
        branch: git_value(&root_path, &["branch", "--show-current"]),
        detected_languages,
    })
}

const MAX_LANGUAGE_DISCOVERY_FILES: usize = 25_000;
const MAX_LANGUAGE_DISCOVERY_DIRECTORIES: usize = 5_000;
const LANGUAGE_DISCOVERY_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".enforce",
    ".ledger",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".venv",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
];

fn detect_project_languages(root: &Path) -> Vec<String> {
    let registry = language_registry();
    let mut detected_languages = BTreeMap::<String, usize>::new();
    let mut directories = vec![root.to_path_buf()];
    let mut visited_directories = 0;
    let mut visited_files = 0;

    while let Some(directory) = directories.pop() {
        if visited_directories >= MAX_LANGUAGE_DISCOVERY_DIRECTORIES
            || visited_files >= MAX_LANGUAGE_DISCOVERY_FILES
        {
            break;
        }
        visited_directories += 1;
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if visited_files >= MAX_LANGUAGE_DISCOVERY_FILES {
                break;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                let ignored = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| LANGUAGE_DISCOVERY_IGNORED_DIRS.contains(&name));
                if !ignored {
                    directories.push(path);
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            visited_files += 1;
            if let Some(language) = language_for_project_path(&path, &registry) {
                *detected_languages.entry(language.to_owned()).or_default() += 1;
            }
        }
    }

    // A manifest is useful for a newly created project before its first source file exists.
    if root.join("Cargo.toml").is_file() {
        detected_languages.entry("rust".to_owned()).or_default();
    }
    if root.join("package.json").is_file() || root.join("tsconfig.json").is_file() {
        detected_languages
            .entry("typescript".to_owned())
            .or_default();
    }
    if root.join("pyproject.toml").is_file() || root.join("requirements.txt").is_file() {
        detected_languages.entry("python".to_owned()).or_default();
    }
    if root.join("terraform").is_dir() || root.join(".github").is_dir() {
        detected_languages.entry("iac".to_owned()).or_default();
    }
    let mut languages = detected_languages.into_iter().collect::<Vec<_>>();
    languages.sort_by(
        |(left_language, left_count), (right_language, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left_language.cmp(right_language))
        },
    );
    languages
        .into_iter()
        .map(|(language, _)| language)
        .collect()
}

fn language_for_project_path<'a>(path: &Path, registry: &'a [LanguageSpec]) -> Option<&'a str> {
    let basename = path.file_name()?.to_string_lossy();
    let extension = path.extension().and_then(|value| value.to_str());
    registry
        .iter()
        .find(|spec| {
            spec.basenames
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&basename))
                || extension.is_some_and(|value| {
                    spec.extensions
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(value))
                })
        })
        .map(|spec| spec.id)
}

#[tauri::command]
fn list_proof_artifacts(root: String) -> Result<Vec<ProofArtifactPayload>, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let proof_root = root_path.join("proof");
    if !proof_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut artifacts = Vec::new();
    let mut stack = vec![proof_root.clone()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).map_err(|error| {
            format!(
                "cannot read proof directory {}: {error}",
                directory.display()
            )
        })? {
            let entry =
                entry.map_err(|error| format!("cannot read proof directory entry: {error}"))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|error| {
                format!("cannot inspect proof artifact {}: {error}", path.display())
            })?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let metadata = entry
                    .metadata()
                    .map_err(|error| format!("cannot read proof artifact metadata: {error}"))?;
                let modified_at = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or_else(
                        || "unknown".to_owned(),
                        |duration| format!("unix:{}", duration.as_secs()),
                    );
                let relative = path
                    .strip_prefix(&root_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                artifacts.push(ProofArtifactPayload {
                    path: relative,
                    modified_at,
                    bytes: metadata.len(),
                });
            }
        }
    }
    artifacts.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.path.cmp(&right.path))
    });
    artifacts.truncate(200);
    Ok(artifacts)
}

#[tauri::command]
fn load_project_proof_snapshot(root: String) -> Result<ProjectProofSnapshot, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    read_project_proof_snapshot(&root_path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod desktop_project_tests {
    use super::{
        activate_security_profile, create_hub_claim,
        desktop_scan_run_path, desktop_workspace_root, detect_project_languages,
        discover_scan_targets, evaluate_rule_path_match,
        load_desktop_rule_catalog, load_project_rule_coverage,
        load_desktop_scan_history, load_engine_capabilities, load_harness_run_detail_from,
        load_harness_runs_from, load_memory_summary, load_workpack_index, parse_workpack_index,
        persist_desktop_report, run_legacy_analysis_sync, write_scan_scope_settings,
        DesktopReportPayload,
        HubClaimRequest, LegacyAnalysisKind, LegacyAnalysisRunPayload, PackagedScanPayload,
        ScanFindingPayload, SecurityActivationRequest,
    };
    use super::project_settings::ScanScopeSettingsRequest;
    use super::project_registry::{
        desktop_project_registration_preview, discover_git_worktrees, load_desktop_projects_from,
        parse_git_worktree_porcelain, paths_equal, register_desktop_project_at, DesktopProject,
    };
    use std::path::Path;

    fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .status()?;
        if !status.success() {
            return Err(format!("git {:?} failed", args).into());
        }
        Ok(())
    }

    fn fixture_project(root: String) -> DesktopProject {
        DesktopProject {
            id: "controlled-fixture".to_owned(),
            name: "Controlled fixture".to_owned(),
            root,
            repo_key: "controlled-fixture".to_owned(),
            kind: "external".to_owned(),
            main_root: None,
            branch: "fixture".to_owned(),
            worktree: "controlled".to_owned(),
            indexed: "missing".to_owned(),
            detected_languages: vec!["rust".to_owned()],
            inspection: Some("configured".to_owned()),
        }
    }

    #[test]
    fn engine_capability_catalog_keeps_planned_dispatch_honest() {
        let payload = load_engine_capabilities();
        let dispatch = payload
            .capabilities
            .iter()
            .find(|capability| capability.id == "dispatch")
            .expect("dispatch capability is present");

        assert!(payload.capabilities.len() >= 12);
        assert_eq!(dispatch.state, "planned");
        assert_eq!(dispatch.target.as_ref().map(|target| target.workspace), Some("findings"));
        assert_eq!(dispatch.target.as_ref().map(|target| target.project_context), Some("required"));
        assert!(dispatch.missing.contains("FixIntent"));
    }

    #[test]
    fn engine_capability_catalog_reports_finding_action_foundation_without_promoting_it() {
        let payload = load_engine_capabilities();
        let actions = payload
            .capabilities
            .iter()
            .find(|capability| capability.id == "finding-actions")
            .expect("finding actions capability is present");

        assert_eq!(actions.state, "partial");
        assert_eq!(actions.target.as_ref().map(|target| target.workspace), Some("findings"));
        assert!(actions.source.contains("typed Rust g03 waiver command"));
        assert!(actions.source.contains("packaged scan overlay"));
        assert!(actions.controls.contains("typed g03 module"));
        assert!(actions.missing.contains("FixIntent lifecycle"));
    }

    #[test]
    fn engine_capability_workpacks_all_exist_in_the_plan_index(
    ) -> Result<(), super::DesktopCommandError> {
        // Every workpack id a capability cites as its provenance must be a real
        // row in the authored plan index, so the capability catalog cannot
        // claim lineage from a workpack that does not exist.
        let payload = load_engine_capabilities();
        let index = load_workpack_index().map_err(super::DesktopCommandError)?;
        let known: std::collections::BTreeSet<&str> =
            index.rows.iter().map(|row| row.id.as_str()).collect();
        let missing: Vec<String> = payload
            .capabilities
            .iter()
            .flat_map(|capability| {
                capability
                    .workpacks
                    .iter()
                    .filter(|workpack| !known.contains(**workpack))
                    .map(move |workpack| format!("{} -> {}", capability.id, workpack))
            })
            .collect();
        assert!(
            missing.is_empty(),
            "engine capabilities reference workpack ids absent from WORKPACK_INDEX.md: {missing:?}"
        );
        Ok(())
    }

    #[test]
    fn engine_capability_catalog_distinguishes_display_catalog_from_runtime_registry() {
        let payload = load_engine_capabilities();
        let rules = payload
            .capabilities
            .iter()
            .find(|capability| capability.id == "rules")
            .expect("rules capability is present");

        assert_eq!(rules.state, "partial");
        assert!(rules.source.contains("display catalog"));
        assert!(rules.missing.contains("production RuleRegistry"));
    }

    #[test]
    fn engine_capability_catalog_routes_project_lifecycle_to_setup_honestly() {
        let payload = load_engine_capabilities();
        let lifecycle = payload
            .capabilities
            .iter()
            .find(|capability| capability.id == "project-lifecycle")
            .expect("project lifecycle capability is present");

        assert_eq!(lifecycle.state, "partial");
        assert_eq!(lifecycle.target.as_ref().map(|target| target.workspace), Some("setup"));
        assert_eq!(lifecycle.target.as_ref().map(|target| target.project_context), Some("required"));
        assert!(lifecycle.workpacks.contains(&"f02"));
        assert!(lifecycle.workpacks.contains(&"f03"));
        assert!(lifecycle.workpacks.contains(&"c11"));
        assert!(lifecycle.missing.contains("onboarding/baseline"));
        assert!(lifecycle.missing.contains("CI wiring"));
        assert!(!lifecycle.source.contains("harness discovery"));
    }

    #[test]
    fn engine_capability_catalog_routes_global_harness_adapters_to_hub() {
        let payload = load_engine_capabilities();
        let adapters = payload
            .capabilities
            .iter()
            .find(|capability| capability.id == "harness-adapters")
            .expect("harness adapters capability is present");

        assert_eq!(adapters.state, "partial");
        assert_eq!(adapters.target.as_ref().map(|target| target.mode), Some("hub"));
        assert_eq!(adapters.target.as_ref().map(|target| target.workspace), Some("hub"));
        assert_eq!(adapters.target.as_ref().and_then(|target| target.subview), Some("harnesses"));
        assert_eq!(adapters.target.as_ref().map(|target| target.project_context), Some("none"));
        assert!(adapters.controls.contains("Hub -> Adapters"));
        assert!(adapters.controls.contains("capability evidence"));
        assert!(adapters.missing.contains("hook installation"));
    }

    #[test]
    fn engine_capability_catalog_keeps_planning_with_engine_workpacks() {
        let payload = load_engine_capabilities();
        let planning = payload
            .capabilities
            .iter()
            .find(|capability| capability.id == "planning")
            .expect("planning capability is present");

        assert_eq!(planning.state, "partial");
        assert_eq!(planning.target.as_ref().map(|target| target.workspace), Some("engine"));
        assert_eq!(planning.target.as_ref().map(|target| target.project_context), Some("none"));
        assert!(planning.controls.contains("Engine Workpacks"));
        assert!(planning.workpacks.contains(&"b01"));
        assert!(planning.workpacks.contains(&"d08"));
        assert!(planning.missing.contains("Plan scaffold/validation UI"));
    }

    #[test]
    fn scan_target_catalog_uses_workspace_only_without_cargo_manifest(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root =
            std::env::temp_dir().join(format!("enforcer-scan-target-test-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;

        let targets = discover_scan_targets(&root)?;

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, "workspace");
        assert_eq!(targets[0].mode, "workspace");
        assert!(targets[0].crate_name.is_none());
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn scan_target_catalog_lists_project_directories_without_ignored_folders(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-scan-directory-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("packages"))?;
        std::fs::create_dir_all(root.join("node_modules"))?;
        std::fs::create_dir_all(root.join("target"))?;
        std::fs::create_dir_all(root.join(".enforce"))?;

        let targets = discover_scan_targets(&root)?;
        let target_ids = targets
            .iter()
            .map(|target| target.id.as_str())
            .collect::<Vec<_>>();

        assert!(target_ids.contains(&"paths:src"));
        assert!(target_ids.contains(&"paths:packages"));
        assert!(!target_ids.contains(&"paths:node_modules"));
        assert!(!target_ids.contains(&"paths:target"));
        assert!(!target_ids.contains(&"paths:.enforce"));
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn scan_target_catalog_lists_real_cargo_packages() -> Result<(), Box<dyn std::error::Error>> {
        let targets = discover_scan_targets(&desktop_workspace_root())?;
        let memory = targets
            .iter()
            .find(|target| target.id == "crate:enforcer-memory")
            .expect("enforcer-memory package target is present");

        assert_eq!(memory.mode, "crate");
        assert_eq!(memory.crate_name.as_deref(), Some("enforcer-memory"));
        Ok(())
    }

    #[test]
    fn scan_target_catalog_lists_controlled_fixture_packages(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = desktop_workspace_root().join(
            "crates/enforcer-ui/frontend/src-tauri/tests/fixtures/desktop/cargo-workspace",
        );
        let targets = discover_scan_targets(&root)?;
        let alpha = targets
            .iter()
            .find(|target| target.id == "crate:desktop-scan-alpha")
            .expect("controlled alpha package target is present");
        let beta = targets
            .iter()
            .find(|target| target.id == "crate:desktop-scan-beta")
            .expect("controlled beta package target is present");

        assert_eq!(alpha.mode, "crate");
        assert_eq!(alpha.crate_name.as_deref(), Some("desktop-scan-alpha"));
        assert_eq!(beta.mode, "crate");
        assert_eq!(beta.crate_name.as_deref(), Some("desktop-scan-beta"));
        Ok(())
    }

    #[test]
    fn packaged_scan_fixture_excludes_other_packages() -> Result<(), Box<dyn std::error::Error>> {
        let fixture_root = desktop_workspace_root().join(
            "crates/enforcer-ui/frontend/src-tauri/tests/fixtures/desktop/cargo-workspace",
        );
        let fixture_root = fixture_root.display().to_string();
        let script = desktop_workspace_root()
            .join("scripts")
            .join("ocentra-enforcer.mjs");
        let scan = |args: &[&str]| -> Result<serde_json::Value, Box<dyn std::error::Error>> {
            let output = std::process::Command::new("node")
                .arg(&script)
                .arg("scan")
                .args(args)
                .current_dir(desktop_workspace_root())
                .output()?;
            assert!(
                !output.status.success(),
                "the controlled fixture must retain scanner findings"
            );
            Ok(serde_json::from_slice(&output.stdout)?)
        };

        let alpha = scan(&[
            "--root",
            &fixture_root,
            "--crate",
            "desktop-scan-alpha",
            "--json",
        ])?;
        assert_eq!(alpha["scope"]["mode"].as_str(), Some("crate"));
        assert_eq!(
            alpha["scope"]["crateName"].as_str(),
            Some("desktop-scan-alpha")
        );
        assert!(alpha["scope"]["files"]
            .as_array()
            .expect("crate report includes selected files")
            .iter()
            .all(|file| file
                .as_str()
                .is_some_and(|path| path.starts_with("crates/scan-alpha/"))));
        assert!(alpha["findings"]
            .as_array()
            .expect("crate report includes findings")
            .iter()
            .all(|finding| finding["file"]
                .as_str()
                .is_some_and(|path| !path.starts_with("crates/scan-beta/"))));

        let workspace = scan(&["--root", &fixture_root, "--workspace", "--json"])?;
        assert_eq!(workspace["scope"]["mode"].as_str(), Some("all"));
        assert!(workspace["findings"]
            .as_array()
            .expect("workspace report includes findings")
            .iter()
            .any(|finding| finding["file"]
                .as_str()
                .is_some_and(|path| path.starts_with("crates/scan-beta/"))));
        Ok(())
    }

    #[test]
    fn desktop_rule_catalog_reads_the_canonical_registry() -> Result<(), Box<dyn std::error::Error>> {
        let catalog = load_desktop_rule_catalog()?;

        assert_eq!(catalog.schema_version, 2);
        assert!(catalog.languages.iter().any(|language| language == "rust"));
        assert!(catalog.rules.iter().any(|rule| rule.id == "RR-12.17"));
        assert!(catalog.rules.iter().any(|rule| rule.id == "TS-1.1"));
        Ok(())
    }

    #[test]
    fn project_rule_coverage_uses_rust_language_scope_and_evaluated_path_status(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = desktop_workspace_root()
            .join("crates/enforcer-ui/frontend/src-tauri/tests/fixtures/desktop/cargo-workspace");
        let coverage = load_project_rule_coverage(root.display().to_string())?;

        assert!(coverage.detected_languages.contains(&"rust".to_owned()));
        assert!(coverage.catalog_languages.contains(&"rust".to_owned()));
        assert!(coverage.rules.iter().any(|rule| rule.language == "rust" && rule.scope == "language-match"));
        assert!(coverage
            .rules
            .iter()
            .any(|rule| rule.language == "rust" && rule.path_match_status == "matched" && rule.matched_path_count > 0));
        Ok(())
    }

    #[test]
    fn rule_path_match_evaluator_reports_match_no_match_and_invalid_patterns() {
        let project_paths = vec!["src/lib.rs".to_owned(), "docs/guide.md".to_owned()];

        let matched = evaluate_rule_path_match(&["**/*.rs".to_owned()], &project_paths);
        assert_eq!(matched.status, "matched");
        assert_eq!(matched.matched_path_count, 1);

        let no_match = evaluate_rule_path_match(&["**/*.py".to_owned()], &project_paths);
        assert_eq!(no_match.status, "no-match");
        assert_eq!(no_match.matched_path_count, 0);

        let invalid = evaluate_rule_path_match(&["[".to_owned()], &project_paths);
        assert_eq!(invalid.status, "invalid-pattern");
    }

    #[test]
    fn project_inspection_observes_registry_languages_without_entering_ignored_trees(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-project-language-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::create_dir_all(root.join("node_modules/package"))?;
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n")?;
        std::fs::write(root.join("src/lib.rs"), "pub fn fixture() {}\n")?;
        std::fs::write(root.join("service.go"), "package fixture\n")?;
        std::fs::write(root.join("script.ps1"), "Write-Output fixture\n")?;
        std::fs::write(root.join("Dockerfile"), "FROM scratch\n")?;
        std::fs::write(
            root.join("node_modules/package/hidden.py"),
            "print('hidden')\n",
        )?;

        let languages = detect_project_languages(&root);

        assert!(languages.contains(&"rust".to_owned()));
        assert!(languages.contains(&"go".to_owned()));
        assert!(languages.contains(&"powershell".to_owned()));
        assert!(languages.contains(&"dockerfile".to_owned()));
        assert!(!languages.contains(&"python".to_owned()));
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn workpack_index_parser_preserves_declared_routing_fields() {
        let rows = parse_workpack_index(
            "| Status | Workpack | Track | owns | owns disjoint? | tier | deps | parallel-safe with |\n\
             |--------|----------|-------|------|----------------|------|------|--------------------|\n\
             | PROOF | [g02 Scan Report Ui](./workpacks/g02-scan-report-ui.md) | G | `crates/enforcer-ui/src/report/` | Y | P3 | g01, f01 | g05, g06 |\n",
        );

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "g02");
        assert_eq!(rows[0].title, "Scan Report Ui");
        assert_eq!(rows[0].status, "PROOF");
        assert_eq!(rows[0].track, "G");
        assert_eq!(rows[0].dependencies, "g01, f01");
        assert_eq!(rows[0].source_path, "workpacks/g02-scan-report-ui.md");
    }

    #[test]
    fn workpack_index_loads_the_authoritative_plan_document() {
        let payload = load_workpack_index().expect("authoritative workpack index loads");

        assert!(payload.rows.len() >= 100);
        assert!(payload.rows.iter().any(|row| row.id == "g09"));
        assert!(payload.status_counts.contains_key("TODO"));
        assert!(payload.caveat.contains("not execution"));
    }

    #[test]
    fn legacy_analysis_bridge_runs_typed_reports_against_the_controlled_fixture(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fixture = desktop_workspace_root()
            .join("crates")
            .join("enforcer-memory")
            .join("tests")
            .join("fixtures")
            .join("memory")
            .join("feature_parity")
            .join("repo");
        let root = fixture.display().to_string();

        let doctrine = run_legacy_analysis_sync(root.clone(), LegacyAnalysisKind::TestDoctrine)?;
        match doctrine {
            LegacyAnalysisRunPayload::TestDoctrine { metadata, report } => {
                assert_eq!(metadata.state, "partial");
                assert_eq!(report.root, root);
                assert!(report.detected.contains_key("unit"));
                assert!(report.summary.categories_relevant > 0);
            }
            LegacyAnalysisRunPayload::UiLogicCoupling { .. } => {
                panic!("test-doctrine request returned UI coupling")
            }
        }

        let coupling = run_legacy_analysis_sync(root, LegacyAnalysisKind::UiLogicCoupling)?;
        match coupling {
            LegacyAnalysisRunPayload::UiLogicCoupling { metadata, report } => {
                assert_eq!(metadata.runtime, "legacy-node-analysis-bridge");
                assert_eq!(report.rule.id, "ARCH-1.16");
                assert_eq!(report.summary.total_findings, report.findings.len());
            }
            LegacyAnalysisRunPayload::TestDoctrine { .. } => {
                panic!("UI coupling request returned test doctrine")
            }
        }
        Ok(())
    }

    #[test]
    fn harness_run_read_model_exposes_typed_diagnostics_and_redacted_artifacts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-desktop-harness-runs-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(&root)?;
        let config = enforcer_harness::config::HarnessConfig::default();
        enforcer_harness::storage::record_run(
            &enforcer_harness::storage::RunInput {
                repo_root: &root,
                run_id: "fixture-failed-run".to_owned(),
                tool: "cargo".to_owned(),
                language: Some("rust".to_owned()),
                command: vec!["cargo".to_owned(), "test".to_owned()],
                stdout: "token AKIAIOSFODNN7EXAMPLE must redact".to_owned(),
                stderr: "fixture failure".to_owned(),
                exit_code: 1,
                crate_name: None,
                package_name: None,
                domain: None,
                tags: vec!["fixture".to_owned()],
                pinned: true,
                started_at: "2026-07-10T00:00:00Z".to_owned(),
                ended_at: "2026-07-10T00:00:01Z".to_owned(),
            },
            &config,
        )?;

        let list = load_harness_runs_from(&root)?;
        assert_eq!(list.runs.len(), 1);
        assert_eq!(list.runs[0].run_id, "fixture-failed-run");
        assert_eq!(
            list.last_failure
                .as_ref()
                .map(|failure| failure.run.status.as_str()),
            Some("failed")
        );
        let detail = load_harness_run_detail_from(&root, "fixture-failed-run")?;
        assert!(detail.stdout.available);
        assert!(!detail.stdout.content.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!detail.diagnostics.is_empty());
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn desktop_registry_round_trips_a_controlled_project() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = std::env::current_dir()?.display().to_string();
        let registry_root = std::env::temp_dir().join(format!(
            "enforcer-desktop-project-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let registry = registry_root.join("desktop-projects.json");

        let registered = register_desktop_project_at(&registry, fixture_project(root))?;
        let reloaded = load_desktop_projects_from(&registry)?;

        assert_eq!(registered.len(), 1);
        assert_eq!(reloaded, registered);
        assert_eq!(reloaded[0].name, "Controlled fixture");
        std::fs::remove_dir_all(registry_root)?;
        Ok(())
    }

    #[test]
    fn desktop_registry_rejects_a_worktree_without_its_main_root() {
        let mut project = fixture_project(
            std::env::current_dir()
                .expect("current directory is available")
                .display()
                .to_string(),
        );
        project.kind = "worktree".to_owned();
        project.main_root = None;
        let registry = std::env::temp_dir().join("enforcer-desktop-project-invalid.json");

        let error = register_desktop_project_at(&registry, project)
            .expect_err("missing worktree main root must be rejected");

        assert!(error.contains("requires mainRoot"));
    }

    #[test]
    fn git_worktree_discovery_reports_primary_linked_and_detached_states(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-worktree-discovery-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let linked = root.with_extension("linked");
        std::fs::create_dir_all(&root)?;
        git(&root, &["init"])?;
        std::fs::write(root.join("README.md"), "fixture\n")?;
        git(&root, &["add", "README.md"])?;
        git(
            &root,
            &[
                "-c",
                "user.email=enforcer@example.test",
                "-c",
                "user.name=Enforcer fixture",
                "commit",
                "-m",
                "fixture",
            ],
        )?;
        git(&root, &["branch", "fixture-linked"])?;
        git(
            &root,
            &[
                "worktree",
                "add",
                linked.to_string_lossy().as_ref(),
                "fixture-linked",
            ],
        )?;

        let discovered = discover_git_worktrees(&root)?;

        assert_eq!(discovered.len(), 2);
        assert_eq!(discovered[0].root, root);
        assert_eq!(discovered[1].root, linked);
        assert_eq!(discovered[1].branch, "fixture-linked");
        assert_eq!(
            parse_git_worktree_porcelain("worktree C:/repo\nHEAD abc\ndetached\n\n")?[0].branch,
            "detached"
        );

        git(
            &root,
            &[
                "worktree",
                "remove",
                "--force",
                linked.to_string_lossy().as_ref(),
            ],
        )?;
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn project_registration_preview_derives_linked_worktree_metadata(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-project-preview-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let linked = root.with_extension("linked");
        std::fs::create_dir_all(&root)?;
        git(&root, &["init"])?;
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n")?;
        git(&root, &["add", "Cargo.toml"])?;
        git(
            &root,
            &[
                "-c",
                "user.email=enforcer@example.test",
                "-c",
                "user.name=Enforcer fixture",
                "commit",
                "-m",
                "fixture",
            ],
        )?;
        git(&root, &["branch", "fixture-linked"])?;
        git(
            &root,
            &[
                "worktree",
                "add",
                linked.to_string_lossy().as_ref(),
                "fixture-linked",
            ],
        )?;

        let preview = desktop_project_registration_preview(&linked)?;

        assert_eq!(preview.project.kind, "worktree");
        assert_eq!(preview.project.worktree, "linked");
        assert_eq!(preview.project.branch, "fixture-linked");
        assert!(paths_equal(
            Path::new(preview.project.main_root.as_deref().expect("main root")),
            &root
        ));
        assert!(preview
            .project
            .detected_languages
            .contains(&"rust".to_owned()));
        assert!(preview
            .project
            .detected_languages
            .contains(&"toml".to_owned()));
        assert_eq!(preview.git_worktree_count, 2);

        git(
            &root,
            &[
                "worktree",
                "remove",
                "--force",
                linked.to_string_lossy().as_ref(),
            ],
        )?;
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn desktop_scan_history_preserves_each_packaged_report_snapshot(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-desktop-scan-history-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(&root)?;
        let report = DesktopReportPayload {
            ok: false,
            scope: "workspace".to_owned(),
            violations: vec![ScanFindingPayload {
                rule_id: "RR-6.1".to_owned(),
                severity: "error".to_owned(),
                title: "Fixture finding".to_owned(),
                detail: "fixture detail".to_owned(),
                file: "src/lib.rs".to_owned(),
                line: 1,
                snippet: None,
                doc: Some("rules/rust/domain.md#covered-rules".to_owned()),
                waiver_id: None,
                waiver_owner: None,
                waiver_reason: None,
                waiver_expires: None,
                waiver_source: None,
            }],
            warnings: Vec::new(),
            waived: Vec::new(),
            total_count: 1,
            runtime: "packaged-enforcer-command".to_owned(),
            persistence: "desktop-cached-packaged-report".to_owned(),
            generated_at: "unix:1".to_owned(),
            run_id: "desktop-scan-1".to_owned(),
            target_label: "Entire workspace".to_owned(),
        };

        persist_desktop_report(&root, &report)?;
        let mut second_report = report.clone();
        second_report.generated_at = "unix:2".to_owned();
        second_report.run_id = "desktop-scan-2".to_owned();
        second_report.warnings = vec![ScanFindingPayload {
            rule_id: "DOC-1.1".to_owned(),
            severity: "warning".to_owned(),
            title: "Fixture warning".to_owned(),
            detail: "fixture warning detail".to_owned(),
            file: "src/lib.rs".to_owned(),
            line: 2,
            snippet: None,
            doc: Some("rules/common/documentation.md#covered-rules".to_owned()),
            waiver_id: None,
            waiver_owner: None,
            waiver_reason: None,
            waiver_expires: None,
            waiver_source: None,
        }];
        second_report.total_count = 2;
        persist_desktop_report(&root, &second_report)?;
        let history = load_desktop_scan_history(root.display().to_string())?;

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].run_id, "desktop-scan-2");
        assert_eq!(history[0].warning_count, 1);
        assert_eq!(history[1].run_id, "desktop-scan-1");
        assert_eq!(history[1].blocking_count, 1);
        assert!(desktop_scan_run_path(&root, "desktop-scan-1").is_file());
        assert!(desktop_scan_run_path(&root, "desktop-scan-2").is_file());
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn packaged_scan_payload_preserves_waiver_audit_fields(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload: PackagedScanPayload = serde_json::from_str(
            r#"{
                "ok": true,
                "violations": [],
                "warnings": [],
                "waived": [{
                    "ruleId": "DOC-1.1",
                    "severity": "warning",
                    "title": "Document public API",
                    "detail": "The exported item has no documentation.",
                    "file": "src/client.ts",
                    "line": 4,
                    "snippet": "export const client = {}",
                    "doc": "rules/common/documentation.md#covered-rules",
                    "waiverId": "fixture-doc-waiver",
                    "waiverOwner": "fixture-maintainer",
                    "waiverReason": "Exercise the desktop waived-finding audit state.",
                    "waiverExpires": "2026-12-31",
                    "waiverSource": "project-registry"
                }],
                "scope": { "mode": "all" }
            }"#,
        )?;

        let finding = &payload.waived[0];
        assert_eq!(finding.waiver_id.as_deref(), Some("fixture-doc-waiver"));
        assert_eq!(finding.waiver_owner.as_deref(), Some("fixture-maintainer"));
        assert_eq!(finding.waiver_expires.as_deref(), Some("2026-12-31"));
        assert_eq!(finding.waiver_source.as_deref(), Some("project-registry"));
        Ok(())
    }

    #[test]
    fn desktop_security_profile_activation_persists_intent_without_claiming_coverage(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-desktop-security-activation-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(&root)?;

        let profile = activate_security_profile(
            root.display().to_string(),
            SecurityActivationRequest {
                source_spec: "docs/money-controls.md".to_owned(),
                owner: "platform-security".to_owned(),
                reason: "the fixture handles money-critical operations".to_owned(),
            },
        )?;

        assert!(profile.activated);
        assert_eq!(profile.profile_name, "money-critical-security");
        assert!(profile.project_activation.contains("platform-security"));
        assert!(profile
            .project_activation
            .contains("docs/money-controls.md"));
        assert!(profile.project_activation.contains("not implemented"));
        assert!(root.join(".enforce/security-profile.json").is_file());
        let activation = enforcer_security::activation::load_project_activation(&root)?
            .expect("activation record");
        assert_eq!(activation.owner, "platform-security");
        assert_eq!(activation.source_spec, "docs/money-controls.md");
        assert_eq!(
            activation.reason,
            "the fixture handles money-critical operations"
        );
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn scan_scope_initialization_persists_typed_project_relative_patterns(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-scan-scope-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(&root)?;
        let settings = write_scan_scope_settings(
            root.display().to_string(),
            ScanScopeSettingsRequest {
                profile_name: "strict".to_owned(),
                ignore_dirs: vec!["generated".to_owned()],
                ignore_file_globs: vec!["**/*.snap".to_owned()],
            },
        )?;

        assert!(settings.exists);
        assert_eq!(settings.profile_name, "strict");
        assert_eq!(settings.ignore_dirs, vec!["generated".to_owned()]);
        assert_eq!(settings.ignore_file_globs, vec!["**/*.snap".to_owned()]);
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn hub_claim_dispatches_one_project_relative_finding_path(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-desktop-hub-claim-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let project = root.join("project");
        let ledger = root.join("ledger");
        std::fs::create_dir_all(project.join("src"))?;
        std::fs::write(project.join("src/lib.rs"), "// fixture\n")?;
        let hub_name = "desktop-fixture-hub".parse()?;
        let lane = "desktop-fixture-lane".parse()?;
        enforcer_coordination::api::init(&ledger, &hub_name, &lane)?;

        let payload = create_hub_claim(
            HubClaimRequest {
                project_root: project.display().to_string(),
                lane_id: "desktop-fixture-lane".to_owned(),
                path: "src/lib.rs".to_owned(),
                reason: "fixture finding assignment".to_owned(),
            },
            Some(ledger.display().to_string()),
        )?;

        let claims = payload["claims"].as_array().expect("claims array");
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0]["laneId"], "desktop-fixture-lane");
        assert_eq!(claims[0]["paths"][0], "src/lib.rs");
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn memory_summary_uses_engine_x06_proofs_for_a_selected_project() -> Result<(), String> {
        let selected_project = desktop_workspace_root()
            .join("crates/enforcer-ui/frontend/src-tauri/tests/fixtures/desktop/cargo-workspace");
        let summary = load_memory_summary(selected_project.display().to_string())?;

        // g09 memory explorer reports the combined evidence scope: the
        // selected project's own store plus engine proof artifacts.
        assert_eq!(summary.provenance.scope, "project-store-plus-engine-proof");
        assert_eq!(
            summary.provenance.selected_project_root,
            selected_project.display().to_string()
        );
        assert!(summary.provenance.artifact_root.ends_with("proof\\memory"));
        assert!(summary.provenance.generated_at_unix_secs.is_some());
        // The parity artifact is live engine evidence that evolves with x06
        // runs, so assert the structural identity (verdicts partition the
        // tool set) instead of pinning counts that rot.
        assert!(summary.parity.available);
        assert!(summary.parity.tools_total > 0);
        assert_eq!(
            summary.parity.equal
                + summary.parity.better
                + summary.parity.worse
                + summary.parity.incomparable
                + summary.parity.unrunnable,
            summary.parity.tools_total
        );
        // CAST-JUSTIFICATION: row counts are tiny (tens of parity rows), so
        // widening usize -> u64 is lossless on every supported target.
        assert_eq!(summary.parity.rows.len() as u64, summary.parity.tools_total);
        Ok(())
    }
}

#[tauri::command]
fn load_graph(root: String, focus: Option<GraphFocusRequest>) -> Result<GraphPayload, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("project root is not a directory: {root}"));
    }
    let store_root = root_path.join(".enforce").join("memory");
    let normalized_root =
        repo_root(&root).map_err(|error| format!("invalid project root: {error}"))?;
    let store = Store::open(&store_root, &normalized_root).map_err(|error| {
        format!(
            "no memory projection for this project at {}: {error}",
            store_root.display()
        )
    })?;
    let sqlite_path = store.sqlite_path();
    if !sqlite_path.exists() {
        return Err(format!(
            "memory Store exists but has no operational graph projection at {}",
            sqlite_path.display()
        ));
    }
    let operational = OperationalGraph::open_read_only(&sqlite_path)
        .map_err(|error| format!("cannot open memory projection: {error}"))?;
    let nodes = operational
        .nodes_snapshot()
        .map_err(|error| format!("cannot read projected graph nodes: {error}"))?;
    let edges = operational
        .edges_snapshot()
        .map_err(|error| format!("cannot read projected graph edges: {error}"))?;
    let graph = CodeGraph::from_store_projection(&nodes, &edges);
    let snapshot = GraphSnapshot::from_code_graph(&graph);
    Ok(build_projection(&root, snapshot, focus))
}

#[tauri::command]
fn load_graph_source_snippet(
    root: String,
    path: String,
    line: usize,
) -> Result<GraphSourceSnippetPayload, String> {
    let root_path = PathBuf::from(&root);
    let source_path = resolve_project_source_path(&root_path, &path)?;
    let metadata = std::fs::metadata(&source_path).map_err(|error| {
        format!(
            "cannot inspect graph source {}: {error}",
            source_path.display()
        )
    })?;
    if metadata.len() > 1_000_000 {
        return Err(format!(
            "graph source is too large to preview safely: {}",
            source_path.display()
        ));
    }
    let source = std::fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "cannot read graph source {}: {error}",
            source_path.display()
        )
    })?;
    let lines = source.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return Ok(GraphSourceSnippetPayload {
            path,
            line,
            start_line: 1,
            end_line: 1,
            content: "<empty file>".to_owned(),
        });
    }
    let target = line.saturating_sub(1).min(lines.len() - 1);
    let start = target.saturating_sub(3);
    let end = (target + 4).min(lines.len());
    let content = lines[start..end]
        .iter()
        .enumerate()
        .map(|(offset, source_line)| format!("{:>5} | {source_line}", start + offset + 1))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(GraphSourceSnippetPayload {
        path,
        line: target + 1,
        start_line: start + 1,
        end_line: end,
        content,
    })
}

fn resolve_project_source_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let requested = Path::new(relative_path);
    if requested.as_os_str().is_empty()
        || requested.is_absolute()
        || requested.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("graph source path must be project-relative".to_owned());
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve project root {}: {error}", root.display()))?;
    let source_path = canonical_root
        .join(requested)
        .canonicalize()
        .map_err(|error| {
            format!(
                "cannot resolve graph source {relative_path} under {}: {error}",
                canonical_root.display()
            )
        })?;
    if !source_path.starts_with(&canonical_root) || !source_path.is_file() {
        return Err("graph source path must resolve to a file inside the project root".to_owned());
    }
    Ok(source_path)
}

fn walk_repo_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    // Code memory excludes Enforcer's own coordination ledger and generated caches.
    // Hub activity, reports, and generated output have separate UI domains.
    const IGNORED_DIRS: &[&str] = &[
        ".git",
        ".ledger",
        ".enforce",
        ".codebase-memory",
        ".cache",
        ".vite",
        "target",
        "node_modules",
        "dist",
        "coverage",
        "__pycache__",
    ];
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("cannot read directory entry: {error}"))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
            if file_type.is_dir() {
                let name = entry.file_name();
                if IGNORED_DIRS.contains(&name.to_string_lossy().as_ref()) {
                    continue;
                }
                stack.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn store_timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    format!("unix:{seconds}")
}

fn build_projection(
    root: &str,
    snapshot: GraphSnapshot,
    focus: Option<GraphFocusRequest>,
) -> GraphPayload {
    let total_nodes = snapshot.node_count();
    let total_edges = snapshot.edge_count();
    let folder_aggregates = graph_folder_aggregates(&snapshot);
    let focus_query = focus
        .as_ref()
        .map(|request| request.query.trim().to_owned())
        .filter(|query| !query.is_empty());
    let focus_node_id = focus
        .as_ref()
        .and_then(|request| request.node_id.as_deref())
        .filter(|node_id| !node_id.trim().is_empty())
        .map(str::to_owned);
    let focused_file_ids = match focus_node_id.as_deref() {
        Some(node_id) => focused_graph_file_ids_for_node(&snapshot, node_id),
        None => focus_query
            .as_deref()
            .map(|query| focused_graph_file_ids(&snapshot, query))
            .unwrap_or_default(),
    };
    let focus_matched = !focused_file_ids.is_empty();
    let mut nodes = Vec::new();
    let mut included = BTreeSet::new();
    let mut files_by_id = BTreeMap::new();
    let mut symbols_by_name: HashMap<&str, Vec<&str>> = HashMap::new();

    for file in &snapshot.files {
        files_by_id.insert(file.id.as_str(), file.rel_path.as_str());
    }
    for file in snapshot
        .files
        .iter()
        .filter(|file| !focus_query.is_some() || focused_file_ids.contains(&file.id))
        .take(MAX_PROJECTION_FILES)
    {
        included.insert(file.id.clone());
        nodes.push(GraphNodePayload {
            id: file.id.clone(),
            label: file.rel_path.clone(),
            kind: "file".to_owned(),
            path: file.rel_path.clone(),
            line: 1,
            status: if file.text_only {
                "text-only"
            } else {
                "indexed"
            }
            .to_owned(),
        });
    }

    for symbol in &snapshot.symbols {
        symbols_by_name
            .entry(symbol.name.as_str())
            .or_default()
            .push(symbol.id.as_str());
    }
    for symbol in &snapshot.symbols {
        if nodes.len() >= MAX_PROJECTION_NODES || !included.contains(&symbol.file_id) {
            continue;
        }
        included.insert(symbol.id.clone());
        nodes.push(GraphNodePayload {
            id: symbol.id.clone(),
            label: symbol.name.clone(),
            kind: symbol_kind(&symbol.kind).to_owned(),
            path: files_by_id
                .get(symbol.file_id.as_str())
                .copied()
                .unwrap_or_default()
                .to_owned(),
            line: symbol.line,
            status: "indexed".to_owned(),
        });
    }

    let mut edges = Vec::new();
    for symbol in &snapshot.symbols {
        if edges.len() >= MAX_PROJECTION_EDGES {
            break;
        }
        if included.contains(&symbol.file_id) && included.contains(&symbol.id) {
            edges.push(GraphEdgePayload {
                from: symbol.file_id.clone(),
                to: symbol.id.clone(),
                label: "defines".to_owned(),
            });
        }
    }
    for call in &snapshot.calls {
        if edges.len() >= MAX_PROJECTION_EDGES {
            break;
        }
        let Some(targets) = symbols_by_name.get(call.callee.as_str()) else {
            continue;
        };
        let Some(target) = targets.first() else {
            continue;
        };
        if included.contains(&call.from_file_id) && included.contains(*target) {
            edges.push(GraphEdgePayload {
                from: call.from_file_id.clone(),
                to: (*target).to_owned(),
                label: "calls".to_owned(),
            });
        }
    }
    for import in &snapshot.imports {
        if edges.len() >= MAX_PROJECTION_EDGES {
            break;
        }
        let target = files_by_id
            .iter()
            .find_map(|(id, path)| path.ends_with(import.module_path.as_str()).then_some(*id));
        if let Some(target) = target.filter(|id| included.contains(*id)) {
            if included.contains(&import.from_file_id) {
                edges.push(GraphEdgePayload {
                    from: import.from_file_id.clone(),
                    to: target.to_owned(),
                    label: "imports".to_owned(),
                });
            }
        }
    }

    GraphPayload {
        root: root.to_owned(),
        total_nodes,
        total_edges,
        files_indexed: snapshot.files.len(),
        folder_aggregates,
        projection_limited: total_nodes > nodes.len() || total_edges > edges.len(),
        focus_query,
        focus_node_id,
        focus_matched,
        nodes,
        edges,
    }
}

fn graph_folder_aggregates(snapshot: &GraphSnapshot) -> Vec<GraphFolderAggregatePayload> {
    let file_paths = snapshot
        .files
        .iter()
        .map(|file| (file.id.as_str(), file.rel_path.as_str()))
        .collect::<HashMap<_, _>>();
    let mut aggregates = BTreeMap::<String, GraphFolderAggregatePayload>::new();
    let mut add = |file_id: &str, files: usize, symbols: usize, calls: usize| {
        let Some(path) = file_paths.get(file_id) else { return };
        let parts = path.split(['/', '\\']).collect::<Vec<_>>();
        for depth in 1..parts.len() {
            let folder = parts[..depth].join("/");
            let entry = aggregates.entry(folder.clone()).or_insert(GraphFolderAggregatePayload { path: folder, files: 0, symbols: 0, calls: 0 });
            entry.files += files;
            entry.symbols += symbols;
            entry.calls += calls;
        }
    };
    for file in &snapshot.files { add(file.id.as_str(), 1, 0, 0); }
    for symbol in &snapshot.symbols { add(symbol.file_id.as_str(), 0, 1, 0); }
    for call in &snapshot.calls { add(call.from_file_id.as_str(), 0, 0, 1); }
    let mut rows = aggregates.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| right.files.cmp(&left.files).then_with(|| left.path.cmp(&right.path)));
    rows
}

fn focused_graph_file_ids(snapshot: &GraphSnapshot, query: &str) -> BTreeSet<String> {
    let normalized = query.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return BTreeSet::new();
    }
    let mut files = snapshot
        .files
        .iter()
        .filter(|file| file.rel_path.to_ascii_lowercase().contains(&normalized))
        .map(|file| file.id.clone())
        .collect::<BTreeSet<_>>();
    files.extend(
        snapshot
            .symbols
            .iter()
            .filter(|symbol| symbol.name.to_ascii_lowercase().contains(&normalized))
            .map(|symbol| symbol.file_id.clone()),
    );
    files.extend(
        snapshot
            .calls
            .iter()
            .filter(|call| call.callee.to_ascii_lowercase().contains(&normalized))
            .map(|call| call.from_file_id.clone()),
    );
    files
}

fn focused_graph_file_ids_for_node(snapshot: &GraphSnapshot, node_id: &str) -> BTreeSet<String> {
    if let Some(file) = snapshot.files.iter().find(|file| file.id == node_id) {
        return BTreeSet::from([file.id.clone()]);
    }
    snapshot
        .symbols
        .iter()
        .find(|symbol| symbol.id == node_id)
        .map(|symbol| BTreeSet::from([symbol.file_id.clone()]))
        .unwrap_or_default()
}

fn symbol_kind(kind: &GraphSymbolKindSnapshot) -> &'static str {
    match kind {
        GraphSymbolKindSnapshot::Function => "function",
        GraphSymbolKindSnapshot::Method => "method",
        GraphSymbolKindSnapshot::Lambda => "lambda",
        GraphSymbolKindSnapshot::Test => "test",
        GraphSymbolKindSnapshot::Class => "class",
        GraphSymbolKindSnapshot::Struct => "struct",
        GraphSymbolKindSnapshot::Interface => "interface",
        GraphSymbolKindSnapshot::Type => "type",
        GraphSymbolKindSnapshot::TypeAlias => "type-alias",
        GraphSymbolKindSnapshot::Enum => "enum",
        GraphSymbolKindSnapshot::Module => "module",
        GraphSymbolKindSnapshot::Variable => "variable",
        GraphSymbolKindSnapshot::Constant => "constant",
    }
}

#[cfg(test)]
mod graph_projection_tests {
    use super::{
        build_projection, focused_graph_file_ids, focused_graph_file_ids_for_node,
        graph_folder_aggregates, load_graph_source_snippet, symbol_kind, GraphFocusRequest,
        MAX_PROJECTION_FILES,
    };
    use enforcer_memory::artifacts::{
        CallEdgeSnapshot, GraphFileSnapshot, GraphSnapshot, GraphSymbolKindSnapshot,
        GraphSymbolSnapshot,
    };

    #[test]
    fn symbol_projection_preserves_all_indexed_symbol_kinds() {
        let cases = [
            (GraphSymbolKindSnapshot::Function, "function"),
            (GraphSymbolKindSnapshot::Method, "method"),
            (GraphSymbolKindSnapshot::Class, "class"),
            (GraphSymbolKindSnapshot::Struct, "struct"),
            (GraphSymbolKindSnapshot::Interface, "interface"),
            (GraphSymbolKindSnapshot::Enum, "enum"),
            (GraphSymbolKindSnapshot::TypeAlias, "type-alias"),
            (GraphSymbolKindSnapshot::Type, "type"),
            (GraphSymbolKindSnapshot::Module, "module"),
            (GraphSymbolKindSnapshot::Test, "test"),
            (GraphSymbolKindSnapshot::Lambda, "lambda"),
            (GraphSymbolKindSnapshot::Variable, "variable"),
            (GraphSymbolKindSnapshot::Constant, "constant"),
        ];
        for (kind, expected) in cases {
            assert_eq!(symbol_kind(&kind), expected);
        }
    }

    #[test]
    fn focused_projection_selects_files_from_path_symbol_and_call_matches() {
        let snapshot = GraphSnapshot {
            files: vec![
                GraphFileSnapshot {
                    id: "file-api".to_owned(),
                    rel_path: "src/api.rs".to_owned(),
                    text_only: false,
                    content_hash: "api".to_owned(),
                    last_commit: None,
                    change_count: 0,
                    chunk_ids: Vec::new(),
                },
                GraphFileSnapshot {
                    id: "file-worker".to_owned(),
                    rel_path: "src/worker.rs".to_owned(),
                    text_only: false,
                    content_hash: "worker".to_owned(),
                    last_commit: None,
                    change_count: 0,
                    chunk_ids: Vec::new(),
                },
            ],
            symbols: vec![GraphSymbolSnapshot {
                id: "symbol-handler".to_owned(),
                kind: GraphSymbolKindSnapshot::Function,
                name: "handle_request".to_owned(),
                file_id: "file-api".to_owned(),
                line: 4,
                source_body_fingerprint: None,
            }],
            calls: vec![CallEdgeSnapshot {
                from_file_id: "file-worker".to_owned(),
                callee: "run_worker".to_owned(),
                line: 9,
            }],
            ..GraphSnapshot::default()
        };

        assert!(focused_graph_file_ids(&snapshot, "api").contains("file-api"));
        assert!(focused_graph_file_ids(&snapshot, "request").contains("file-api"));
        assert!(focused_graph_file_ids(&snapshot, "worker").contains("file-worker"));
        let exact_symbol_file = focused_graph_file_ids_for_node(&snapshot, "symbol-handler");
        assert!(exact_symbol_file.contains("file-api"));
        assert!(!exact_symbol_file.contains("file-worker"));
        let src = graph_folder_aggregates(&snapshot)
            .into_iter()
            .find(|aggregate| aggregate.path == "src")
            .expect("src aggregate");
        assert_eq!((src.files, src.symbols, src.calls), (2, 1, 1));

        let projection = build_projection(
            "fixture",
            snapshot,
            Some(GraphFocusRequest {
                query: "request".to_owned(),
                node_id: None,
            }),
        );
        assert_eq!(projection.focus_query.as_deref(), Some("request"));
        assert!(projection.focus_matched);
        assert_eq!(projection.nodes.len(), 2);
        assert!(projection
            .nodes
            .iter()
            .all(|node| node.path == "src/api.rs"));
    }

    #[test]
    fn focused_projection_reaches_a_file_outside_the_default_file_cap() {
        let files = (0..=MAX_PROJECTION_FILES)
            .map(|index| GraphFileSnapshot {
                id: format!("file-{index}"),
                rel_path: if index == MAX_PROJECTION_FILES {
                    "src/late-focus.rs".to_owned()
                } else {
                    format!("src/file-{index}.rs")
                },
                text_only: false,
                content_hash: format!("hash-{index}"),
                last_commit: None,
                change_count: 0,
                chunk_ids: Vec::new(),
            })
            .collect();
        let snapshot = GraphSnapshot {
            files,
            ..GraphSnapshot::default()
        };

        let default_projection = build_projection("fixture", snapshot.clone(), None);
        assert!(default_projection.projection_limited);
        assert!(!default_projection
            .nodes
            .iter()
            .any(|node| node.path == "src/late-focus.rs"));

        let focused_projection = build_projection(
            "fixture",
            snapshot,
            Some(GraphFocusRequest {
                query: "late-focus".to_owned(),
                node_id: None,
            }),
        );
        assert!(focused_projection.focus_matched);
        assert_eq!(focused_projection.nodes.len(), 1);
        assert_eq!(focused_projection.nodes[0].path, "src/late-focus.rs");
    }

    #[test]
    fn graph_source_snippet_reads_context_only_inside_the_selected_project(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "enforcer-graph-source-snippet-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("src"))?;
        std::fs::write(
            root.join("src/lib.rs"),
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n",
        )?;

        let snippet =
            load_graph_source_snippet(root.display().to_string(), "src/lib.rs".to_owned(), 5)?;

        assert_eq!(snippet.start_line, 2);
        assert_eq!(snippet.end_line, 8);
        assert!(snippet.content.contains("    5 | five"));
        let error =
            load_graph_source_snippet(root.display().to_string(), "../outside.rs".to_owned(), 1)
                .err()
                .expect("parent path is rejected");
        assert!(error.contains("project-relative"));
        std::fs::remove_dir_all(root)?;
        Ok(())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            desktop_status,
            load_engine_capabilities,
            load_desktop_rule_catalog,
            load_project_rule_coverage,
            load_harness_discovery,
            load_workpack_index,
            load_security_profile,
            activate_security_profile,
            run_packaged_scan,
            waive_packaged_finding,
            load_scan_targets,
            run_legacy_analysis,
            load_harness_runs,
            load_harness_run_detail,
            load_cached_scan,
            load_desktop_scan_history,
            load_desktop_scan_run,
            load_hub,
            send_hub_message,
            acknowledge_hub_message,
            create_hub_claim,
            load_project_settings,
            load_scan_scope_settings,
            write_scan_scope_settings,
            write_rule_override,
            inspect_project,
            list_proof_artifacts,
            load_project_proof_snapshot,
            load_desktop_projects,
            register_desktop_project,
            preview_desktop_project_registration,
            discover_desktop_project_worktrees,
            load_graph,
            load_graph_source_snippet,
            search_memory_graph,
            memory_index_status,
            create_memory_index,
            load_memory_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running Enforcer desktop UI");
}
