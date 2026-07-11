import { type ReactElement, useEffect, useMemo, useState } from "react";
import { AppShell, type WorkspaceKey } from "./components/AppShell";
import { AnalysisWorkspace, type AnalysisRun, type AnalysisRunKind } from "./components/AnalysisWorkspace";
import { RunsWorkspace, type HarnessRunDetailPayload, type HarnessRunPayload } from "./components/RunsWorkspace";
import { CommandBar } from "./components/CommandBar";
import { FindingsWorkspace } from "./components/FindingsWorkspace";
import { HubWorkspace, type HarnessDiscoveryPayload, type HubFindingHandoff, type HubView } from "./components/HubWorkspace";
import { ProjectsWorkspace } from "./components/ProjectsWorkspace";
import { ProjectOverviewWorkspace } from "./components/ProjectOverviewWorkspace";
import { ProjectSetupWorkspace } from "./components/ProjectSetupWorkspace";
import { EngineWorkspace, type EngineCapability, type EngineCapabilityTarget } from "./components/EngineWorkspace";
import { DoctrineWorkspace } from "./components/DoctrineWorkspace";
import { MemoryExplorerWorkspace, type MemorySummaryPayload } from "./components/MemoryExplorerWorkspace";
import { ProjectSettingsWorkspace, type SettingsTab } from "./components/ProjectSettingsWorkspace";
import { ProofWorkspace } from "./components/ProofWorkspace";
import { AssuranceWorkspace, type SecurityProfilePayload } from "./components/AssuranceWorkspace";
import { RuleCatalogWorkspace } from "./components/RuleCatalogWorkspace";
import { appData, type Project } from "./data/enforcerAppData";
import { invokeDesktop } from "./data/desktopClient";
import { displayFindings, emptyReport, type EnforcerReport } from "./data/reportAdapter";
import { graphFromNative, unavailableGraph, type GraphFocus, type GraphNode, type GraphSourceSnippet, type NativeGraphPayload, type ProjectGraph } from "./data/graphAdapter";
import type { CatalogRule, ProjectRuleCoverage, RuleOverride } from "./data/ruleCatalog";

type HubPayload = {
  rootPath: string;
  lanes: Array<{ laneId: string; writers: string[]; statusSummary?: string; heartbeatSummary?: string }>;
  claims: Array<{ claimId: string; laneId: string; writer: string; paths: string[]; reason?: string }>;
  workers: Array<{ writer: string; laneId: string; state?: string; summary?: string; currentTaskId?: string; lastSeenAt: string }>;
  tasks: Array<{ taskId: string; laneId: string; writer: string; state: string; summary: string; updatedAt: string; title?: string; prUrl?: string }>;
  mail: Array<{ messageId: string; fromWriter: string; to?: string; body?: string; ts: string; ackedBy: string[] }>;
  sync: { totalEvents: number; duplicateCount: number; warnings: string[] };
};
type ProjectSettingsPayload = {
  sourcePath: string;
  nativeTies: Array<{ tool: string; mode: string; scope: string }>;
  ruleToggles: Array<{ ruleId: string; enabled: boolean; severity?: string; waiverOwner?: string; waiverReason?: string }>;
};
type ProjectInspectionPayload = {
  available: boolean;
  gitRoot?: string;
  branch?: string;
  detectedLanguages: string[];
};
type GraphSearchPayload = { total: number; hasMore: boolean; query: string; projectScope: string; results: Array<{ nodeId: string; name: string; qualifiedName: string; label: string; filePath: string; evidenceKind: "code-graph" | "learning-memory" | "proof-artifact"; rank?: string }> };
type ProofArtifactPayload = { path: string; modifiedAt: string; bytes: number };
type ProjectProofSnapshot = {
  proofRoot: string;
  currentGit: { commit?: string; branch?: string; dirty?: boolean };
  journal: { path: string; state: "missing" | "verified" | "invalid"; recordCount: number; latestEventType?: string; latestProofId?: string; latestTimestamp?: string; error?: string };
  runs: Array<{ path: string; proofRun?: { proofId: string; runId: string; title: string; capability: string; status: "passed" | "failed" | "manual-required" | "unavailable"; endedAt: string; pinned: boolean; diagnosticCount: number; artifacts: Array<{ path: string; byteLength: number }> }; freshness: "current" | "stale" | "unavailable" | "invalid"; artifacts: { declared: number; present: number; missing: number; totalBytes: number }; parseError?: string }>;
  claim: { registryPath: string; state: "unconfigured" | "invalid-registry" | "no-required-proofs" | "ready" | "blocked"; requiredProofIds: string[]; claim?: { accepted: Array<{ proofId: string; runId: string }>; violations: Array<{ proofId: string; code: string; message: string }> }; error?: string };
};
type ScanScopeSettingsPayload = { sourcePath: string; exists: boolean; profileName: string; ignoreDirs: string[]; ignoreFileGlobs: string[] };
type ProjectDiscoveryPayload = { projects: Project[]; discoveredCount: number; mainRoot: string };
type ProjectRegistrationPreview = { requestedRoot: string; project: Project; topology: string; gitWorktreeCount: number };
type DesktopScanHistoryEntry = { runId: string; generatedAt: string; scope: string; totalCount: number; blockingCount: number; warningCount: number; waivedCount: number; runtime: string; persistence: string };
type WorkpackIndexPayload = {
  sourcePath: string;
  rows: Array<{ id: string; title: string; status: string; track: string; owns: string; tier: string; dependencies: string; parallelSafeWith: string; sourcePath: string }>;
  statusCounts: Record<string, number>;
  caveat: string;
};
type RawTestDoctrineCiState = { wired: boolean; blocking: boolean; evidence: string[] };
type RawTestDoctrineReport = {
  root: string;
  caveat: string;
  ciConfigFilesFound: string[];
  hasUntrackedCiFiles: boolean;
  detected: Record<string, { label: string; present: boolean; relevant: boolean; evidence: string[]; ci: RawTestDoctrineCiState; ciIncludingUntracked?: RawTestDoctrineCiState | null }>;
  missing: Array<{ category: string; label: string; tier: "core" | "suggested" | "optional"; reason: string }>;
  ciGaps: Array<{ category: string; label: string; reason: string; ciEvidence: string[] }>;
  summary: { categoriesRelevant: number; categoriesPresent: number; categoriesMissing: number; coreMissing: number; ciGaps: number };
};
type RawUiLogicCouplingReport = {
  root: string;
  caveat: string;
  rule: { id: string; title: string; doc: string; aka: string; why: string };
  findings: Array<{ file: string; kind: string; severity: "hard" | "info"; source: string; binding: string; hasDataFetchPrimitive?: boolean }>;
  hard: Array<{ file: string; kind: string; severity: "hard" | "info"; source: string; binding: string; hasDataFetchPrimitive?: boolean }>;
  info: Array<{ file: string; kind: string; severity: "hard" | "info"; source: string; binding: string; hasDataFetchPrimitive?: boolean }>;
  summary: { totalFindings: number; hardFindings: number; infoFindings: number; filesWithHardFindings: number };
};
type LegacyAnalysisPayload = {
  analysisKind: "test-doctrine";
  metadata: { generatedAt: string; runtime: string; caveat: string; state: "partial" };
  report: RawTestDoctrineReport;
} | {
  analysisKind: "ui-logic-coupling";
  metadata: { generatedAt: string; runtime: string; caveat: string; state: "partial" };
  report: RawUiLogicCouplingReport;
};

type DesktopScanTarget = {
  id: string;
  label: string;
  description: string;
  mode: "workspace" | "crate" | "files" | "diff";
  crateName?: string;
  files?: string[];
  base?: string;
  head?: string;
};

type DesktopRuleCatalogPayload = { rules: CatalogRule[] };

function analysisRunFromNative(payload: LegacyAnalysisPayload): AnalysisRun {
  if (payload.analysisKind === "test-doctrine") {
    const report = payload.report;
    const status = (value: RawTestDoctrineCiState) => ({ ...value, evidence: value.evidence.map((step) => ({ step, blocking: value.blocking })) });
    return {
      kind: "test-doctrine",
      root: report.root,
      caveat: report.caveat,
      generatedAt: payload.metadata.generatedAt,
      runtime: payload.metadata.runtime,
      detected: Object.fromEntries(Object.entries(report.detected).map(([id, category]) => [id, { ...category, ci: status(category.ci), ciIncludingUntracked: category.ciIncludingUntracked ? status(category.ciIncludingUntracked) : null }])),
      missing: report.missing,
      ciConfigFilesFound: report.ciConfigFilesFound.map((path) => ({ path, tracked: true })),
      hasUntrackedCiFiles: report.hasUntrackedCiFiles,
      ciGaps: report.ciGaps.map((gap) => ({ ...gap, ciEvidence: gap.ciEvidence.map((step) => ({ step, blocking: false })) })),
      summary: report.summary,
    };
  }
  const report = payload.report;
  return {
    kind: "ui-logic-coupling",
    root: report.root,
    caveat: report.caveat,
    generatedAt: payload.metadata.generatedAt,
    runtime: payload.metadata.runtime,
    rule: report.rule,
    findings: report.findings,
    hard: report.hard,
    info: report.info,
    summary: report.summary,
  };
}

function mergeProjects(current: Project[], registered: Project[]): Project[] {
  const merged = new Map(current.map((project) => [project.id, project]));
  for (const project of registered) merged.set(project.id, project);
  return [...merged.values()];
}

function isRuleSeverity(value: string | undefined): value is NonNullable<RuleOverride["severity"]> {
  return value === "error" || value === "warning" || value === "info";
}

function resolveHubView(value: string | undefined): HubView | undefined {
  switch (value) {
    case "lanes":
    case "inbox":
    case "claims":
    case "tasks":
    case "workers":
    case "harnesses":
      return value;
    default:
      return undefined;
  }
}

/** Owns desktop UI state and composes presentation workspaces. */
export function App(): ReactElement {
  const [workspace, setWorkspace] = useState<WorkspaceKey>("projects");
  const [settingsTab, setSettingsTab] = useState<SettingsTab>("scope");
  const [projects, setProjects] = useState<Project[]>(appData.projects);
  const [projectRegistryError, setProjectRegistryError] = useState("");
  const [selectedProjectId, setSelectedProjectId] = useState(appData.projects[0].id);
  const [selectedFindingId, setSelectedFindingId] = useState("");
  const [ruleFocusId, setRuleFocusId] = useState<string>();
  const [nativeShell, setNativeShell] = useState("browser-preview");
  const [bindingMode, setBindingMode] = useState("unavailable");
  const [engineCapabilities, setEngineCapabilities] = useState<EngineCapability[]>();
  const [engineCapabilitiesLoading, setEngineCapabilitiesLoading] = useState(true);
  const [engineCapabilitiesError, setEngineCapabilitiesError] = useState("");
  const [workpackIndex, setWorkpackIndex] = useState<WorkpackIndexPayload>();
  const [workpackIndexLoading, setWorkpackIndexLoading] = useState(true);
  const [workpackIndexError, setWorkpackIndexError] = useState("");
  const [securityProfile, setSecurityProfile] = useState<SecurityProfilePayload>();
  const [securityProfileLoading, setSecurityProfileLoading] = useState(false);
  const [securityProfileError, setSecurityProfileError] = useState("");
  const [graph, setGraph] = useState<ProjectGraph>(() => unavailableGraph(appData.projects[0]));
  const [graphLoading, setGraphLoading] = useState(false);
  const [memorySummary, setMemorySummary] = useState<MemorySummaryPayload>();
  const [memorySummaryLoading, setMemorySummaryLoading] = useState(false);
  const [memoryRevision, setMemoryRevision] = useState(0);
  const [graphFocus, setGraphFocus] = useState<GraphFocus>();
  const [reportsByProject, setReportsByProject] = useState<Record<string, EnforcerReport>>({});
  const [scanHistoryByProject, setScanHistoryByProject] = useState<Record<string, DesktopScanHistoryEntry[]>>({});
  const [scanState, setScanState] = useState<"idle" | "running" | "complete" | "failed">("idle");
  const [scanError, setScanError] = useState("");
  const [scanTargets, setScanTargets] = useState<DesktopScanTarget[]>([]);
  const [scanTargetsLoading, setScanTargetsLoading] = useState(false);
  const [scanTargetsError, setScanTargetsError] = useState("");
  const [analysisKind, setAnalysisKind] = useState<AnalysisRunKind>("test-doctrine");
  const [analysisRunsByProject, setAnalysisRunsByProject] = useState<Record<string, Partial<Record<AnalysisRunKind, AnalysisRun>>>>({});
  const [analysisLoading, setAnalysisLoading] = useState(false);
  const [analysisError, setAnalysisError] = useState("");
  const [harnessRuns, setHarnessRuns] = useState<HarnessRunPayload>();
  const [harnessRunDetail, setHarnessRunDetail] = useState<HarnessRunDetailPayload>();
  const [selectedHarnessRunId, setSelectedHarnessRunId] = useState("");
  const [harnessRunsLoading, setHarnessRunsLoading] = useState(false);
  const [harnessRunsError, setHarnessRunsError] = useState("");
  const [hub, setHub] = useState<HubPayload>();
  const [hubLoading, setHubLoading] = useState(false);
  const [hubError, setHubError] = useState("");
  const [hubFindingHandoff, setHubFindingHandoff] = useState<HubFindingHandoff>();
  const [hubView, setHubView] = useState<HubView>("lanes");
  const [projectSettings, setProjectSettings] = useState<ProjectSettingsPayload>();
  const [projectSettingsLoading, setProjectSettingsLoading] = useState(false);
  const [projectSettingsError, setProjectSettingsError] = useState("");
  const [scanScopeSettings, setScanScopeSettings] = useState<ScanScopeSettingsPayload>();
  const [scanScopeSettingsLoading, setScanScopeSettingsLoading] = useState(false);
  const [scanScopeSettingsError, setScanScopeSettingsError] = useState("");
  const [harnessDiscovery, setHarnessDiscovery] = useState<HarnessDiscoveryPayload>();
  const [harnessDiscoveryLoading, setHarnessDiscoveryLoading] = useState(false);
  const [harnessDiscoveryError, setHarnessDiscoveryError] = useState("");
  const [memorySearch, setMemorySearch] = useState<GraphSearchPayload>();
  const [memorySearchLoading, setMemorySearchLoading] = useState(false);
  const [memorySearchError, setMemorySearchError] = useState("");
  const [proofArtifacts, setProofArtifacts] = useState<ProofArtifactPayload[]>([]);
  const [proofSnapshot, setProofSnapshot] = useState<ProjectProofSnapshot>();
  const [proofArtifactsLoading, setProofArtifactsLoading] = useState(false);
  const [proofArtifactsError, setProofArtifactsError] = useState("");
  const [overridesByProject, setOverridesByProject] = useState<Record<string, RuleOverride[]>>({});
  const [ruleCatalog, setRuleCatalog] = useState<CatalogRule[]>([]);
  const [ruleCatalogLoading, setRuleCatalogLoading] = useState(true);
  const [ruleCatalogError, setRuleCatalogError] = useState("");
  const [projectRuleCoverage, setProjectRuleCoverage] = useState<ProjectRuleCoverage>();
  const assuranceRoot = projects.find((project) => project.id === selectedProjectId)?.root ?? appData.projects[0].root;

  useEffect(() => {
    invokeDesktop<{ shell: string; binding_mode?: string; bindingMode?: string }>("desktop_status")
      .then((status) => {
        setNativeShell(status.shell);
        setBindingMode(status.bindingMode ?? status.binding_mode ?? "unavailable");
      })
      .catch(() => {
        setNativeShell("browser-preview");
        setBindingMode("unavailable");
      });
  }, []);

  useEffect(() => {
    if (workspace !== "assurance") return;
    let cancelled = false;
    setSecurityProfileLoading(true);
    setSecurityProfileError("");
    invokeDesktop<SecurityProfilePayload>("load_security_profile", { root: assuranceRoot })
      .then((payload) => { if (!cancelled) setSecurityProfile(payload); })
      .catch((error) => { if (!cancelled) setSecurityProfileError(`Security profile unavailable: ${String(error)}`); })
      .finally(() => { if (!cancelled) setSecurityProfileLoading(false); });
    return () => { cancelled = true; };
  }, [assuranceRoot, workspace]);

  async function activateSecurityProfile(request: { sourceSpec: string; owner: string; reason: string }) {
    const profile = await invokeDesktop<SecurityProfilePayload>("activate_security_profile", { root: selectedProject.root, request });
    setSecurityProfile(profile);
  }

  async function loadGraphSourceSnippet(node: GraphNode) {
    return invokeDesktop<GraphSourceSnippet>("load_graph_source_snippet", {
      root: selectedProject.root,
      path: node.path,
      line: node.line,
    });
  }

  useEffect(() => {
    let cancelled = false;
    invokeDesktop<WorkpackIndexPayload>("load_workpack_index")
      .then((payload) => { if (!cancelled) setWorkpackIndex(payload); })
      .catch((error) => { if (!cancelled) setWorkpackIndexError(`Workpack index unavailable: ${String(error)}`); })
      .finally(() => { if (!cancelled) setWorkpackIndexLoading(false); });
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    let cancelled = false;
    invokeDesktop<{ capabilities: EngineCapability[] }>("load_engine_capabilities")
      .then(({ capabilities }) => {
        if (!cancelled) setEngineCapabilities(capabilities);
      })
      .catch((error) => {
        if (!cancelled) setEngineCapabilitiesError(`Engine capability map unavailable: ${String(error)}`);
      })
      .finally(() => {
        if (!cancelled) setEngineCapabilitiesLoading(false);
      });
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    let cancelled = false;
    invokeDesktop<DesktopRuleCatalogPayload>("load_desktop_rule_catalog")
      .then((payload) => { if (!cancelled) setRuleCatalog(payload.rules); })
      .catch((error) => { if (!cancelled) setRuleCatalogError(`Rule catalog unavailable: ${String(error)}`); })
      .finally(() => { if (!cancelled) setRuleCatalogLoading(false); });
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    let cancelled = false;
    setProjectRegistryError("");
    invokeDesktop<Project[]>("load_desktop_projects")
      .then((registered) => {
        if (!cancelled) setProjects((current) => mergeProjects(current, registered));
      })
      .catch((error) => {
        if (!cancelled) setProjectRegistryError(`Desktop project registry unavailable: ${String(error)}`);
      });
    return () => { cancelled = true; };
  }, []);

  const projectRoots = projects.map((project) => `${project.id}:${project.root}`).sort().join("|");
  useEffect(() => {
    let cancelled = false;
    Promise.all(projects.map(async (project) => ({ project, inspection: await invokeDesktop<ProjectInspectionPayload>("inspect_project", { root: project.root }) })))
      .then((rows) => {
        if (cancelled) return;
        setProjects((current) => current.map((project) => {
          const row = rows.find((item) => item.project.id === project.id);
          if (!row) return project;
          const liveLanguages = row.inspection.detectedLanguages;
          return {
            ...project,
            branch: row.inspection.branch ?? project.branch,
            detectedLanguages: liveLanguages.length ? liveLanguages : project.detectedLanguages,
            inspection: row.inspection.available ? "live" : "unavailable",
          };
        }));
      })
      .catch(() => {
        if (!cancelled) setProjects((current) => current.map((project) => ({ ...project, inspection: "configured" })));
      });
    return () => { cancelled = true; };
  }, [projectRoots]);

  const selectedProject = useMemo(
    () => projects.find((project) => project.id === selectedProjectId) ?? projects[0],
    [projects, selectedProjectId],
  );

  useEffect(() => {
    let cancelled = false;
    setScanTargetsLoading(true);
    setScanTargetsError("");
    invokeDesktop<DesktopScanTarget[]>("load_scan_targets", { root: selectedProject.root })
      .then((targets) => {
        if (cancelled) return;
        setScanTargets(targets);
      })
      .catch((error) => {
        if (!cancelled) {
          setScanTargets([]);
          setScanTargetsError(`Target discovery unavailable: ${String(error)}`);
        }
      })
      .finally(() => { if (!cancelled) setScanTargetsLoading(false); });
    return () => { cancelled = true; };
  }, [selectedProject.root]);

  useEffect(() => {
    let cancelled = false;
    invokeDesktop<ProjectRuleCoverage>("load_project_rule_coverage", { root: selectedProject.root })
      .then((payload) => { if (!cancelled) setProjectRuleCoverage(payload); })
      .catch(() => { if (!cancelled) setProjectRuleCoverage(undefined); });
    return () => { cancelled = true; };
  }, [selectedProject.root]);
  const projectOverrides = useMemo(() => {
    const persisted: RuleOverride[] = (projectSettings?.ruleToggles ?? []).map((toggle) => ({
      ruleId: toggle.ruleId,
      enabled: toggle.enabled,
      severity: isRuleSeverity(toggle.severity) ? toggle.severity : undefined,
      waiver: toggle.waiverOwner && toggle.waiverReason ? { owner: toggle.waiverOwner, reason: toggle.waiverReason } : undefined,
    }));
    const staged = overridesByProject[selectedProject.id] ?? [];
    return [...persisted.filter((override) => !staged.some((item) => item.ruleId === override.ruleId)), ...staged];
  }, [overridesByProject, projectSettings, selectedProject.id]);
  const report = reportsByProject[selectedProject.id] ?? emptyReport;
  const findings = useMemo(() => displayFindings(report, selectedProject.repoKey), [report, selectedProject.repoKey]);
  const selectedFinding = findings.find((finding) => finding.id === selectedFindingId) ?? findings[0];
  useEffect(() => {
    let cancelled = false;
    setScanState("idle");
    setScanError("");
    invokeDesktop<EnforcerReport | null>("load_cached_scan", { root: selectedProject.root })
      .then((report) => {
        if (cancelled || !report) return;
        setReportsByProject((current) => ({ ...current, [selectedProject.id]: report }));
        setScanState("complete");
      })
      .catch((error) => {
        if (!cancelled) setScanError(`Cached scan report unavailable: ${String(error)}`);
      });
    return () => { cancelled = true; };
  }, [selectedProject.id, selectedProject.root]);

  useEffect(() => {
    let cancelled = false;
    invokeDesktop<DesktopScanHistoryEntry[]>("load_desktop_scan_history", { root: selectedProject.root })
      .then((history) => { if (!cancelled) setScanHistoryByProject((current) => ({ ...current, [selectedProject.id]: history })); })
      .catch(() => { if (!cancelled) setScanHistoryByProject((current) => ({ ...current, [selectedProject.id]: [] })); });
    return () => { cancelled = true; };
  }, [selectedProject.id, selectedProject.root]);

  useEffect(() => {
    let cancelled = false;
    invokeDesktop<{ available: boolean }>("memory_index_status", { root: selectedProject.root })
      .then(({ available }) => {
        if (cancelled) return;
        setProjects((current) => current.map((project) => project.id === selectedProject.id ? { ...project, indexed: available ? "ready" : "missing" } : project));
      })
      .catch(() => {
        if (!cancelled) setProjects((current) => current.map((project) => project.id === selectedProject.id ? { ...project, indexed: "missing" } : project));
      });
    return () => { cancelled = true; };
  }, [selectedProject.id, selectedProject.root]);

  useEffect(() => {
    if (workspace !== "memory") return;
    let cancelled = false;
    setGraphLoading(true);
    invokeDesktop<NativeGraphPayload>("load_graph", { root: selectedProject.root, focus: graphFocus })
      .then((payload) => {
        if (!cancelled) setGraph(graphFromNative(selectedProject, payload));
      })
      .catch((error) => {
        if (!cancelled) setGraph(unavailableGraph(selectedProject, String(error)));
      })
      .finally(() => {
        if (!cancelled) setGraphLoading(false);
      });
    return () => { cancelled = true; };
  }, [graphFocus, memoryRevision, selectedProject, workspace]);

  useEffect(() => {
    if (workspace !== "hub") return;
    let cancelled = false;
    setHubLoading(true);
    setHubError("");
    invokeDesktop<HubPayload>("load_hub")
      .then((payload) => { if (!cancelled) setHub(payload); })
      .catch((error) => { if (!cancelled) setHubError(`Coordination ledger unavailable: ${String(error)}`); })
      .finally(() => { if (!cancelled) setHubLoading(false); });
    return () => { cancelled = true; };
  }, [workspace]);

  useEffect(() => {
    let cancelled = false;
    setProjectSettingsLoading(true);
    setProjectSettingsError("");
    invokeDesktop<ProjectSettingsPayload>("load_project_settings", { root: selectedProject.root })
      .then((payload) => { if (!cancelled) setProjectSettings(payload); })
      .catch((error) => { if (!cancelled) setProjectSettingsError(`Project configuration unavailable: ${String(error)}`); })
      .finally(() => { if (!cancelled) setProjectSettingsLoading(false); });
    return () => { cancelled = true; };
  }, [selectedProject.root]);

  useEffect(() => {
    let cancelled = false;
    setScanScopeSettingsLoading(true);
    setScanScopeSettingsError("");
    invokeDesktop<ScanScopeSettingsPayload>("load_scan_scope_settings", { root: selectedProject.root })
      .then((payload) => { if (!cancelled) setScanScopeSettings(payload); })
      .catch((error) => { if (!cancelled) setScanScopeSettingsError(`Scan policy unavailable: ${String(error)}`); })
      .finally(() => { if (!cancelled) setScanScopeSettingsLoading(false); });
    return () => { cancelled = true; };
  }, [selectedProject.root]);

  async function refreshHarnessDiscovery() {
    setHarnessDiscoveryLoading(true);
    setHarnessDiscoveryError("");
    try {
      setHarnessDiscovery(await invokeDesktop<HarnessDiscoveryPayload>("load_harness_discovery"));
    } catch (error) {
      setHarnessDiscoveryError(`Harness discovery unavailable: ${String(error)}`);
    } finally {
      setHarnessDiscoveryLoading(false);
    }
  }

  useEffect(() => {
    if (workspace !== "hub") return;
    void refreshHarnessDiscovery();
  }, [workspace]);

  useEffect(() => {
    if (workspace !== "runs") return;
    void refreshHarnessRuns();
  }, [selectedProject.root, workspace]);

  useEffect(() => {
    if (workspace !== "proofs" && workspace !== "setup") return;
    let cancelled = false;
    setProofArtifactsLoading(true);
    setProofArtifactsError("");
    Promise.all([
      invokeDesktop<ProjectProofSnapshot>("load_project_proof_snapshot", { root: selectedProject.root }),
      invokeDesktop<ProofArtifactPayload[]>("list_proof_artifacts", { root: selectedProject.root }),
    ])
      .then(([snapshot, artifacts]) => {
        if (cancelled) return;
        setProofSnapshot(snapshot);
        setProofArtifacts(artifacts);
      })
      .catch((error) => { if (!cancelled) setProofArtifactsError(String(error)); })
      .finally(() => { if (!cancelled) setProofArtifactsLoading(false); });
    return () => { cancelled = true; };
  }, [selectedProject.root, workspace]);

  useEffect(() => {
    if (workspace !== "memory") return;
    let cancelled = false;
    setMemorySummaryLoading(true);
    invokeDesktop<MemorySummaryPayload>("load_memory_summary", { root: selectedProject.root })
      .then((payload) => { if (!cancelled) setMemorySummary(payload); })
      .catch(() => { if (!cancelled) setMemorySummary(undefined); })
      .finally(() => { if (!cancelled) setMemorySummaryLoading(false); });
    return () => { cancelled = true; };
  }, [memoryRevision, selectedProject, workspace]);

  async function createMemoryIndex() {
    await invokeDesktop("create_memory_index", { root: selectedProject.root });
    setProjects((current) => current.map((project) => project.id === selectedProject.id ? { ...project, indexed: "ready" } : project));
    setMemoryRevision((current) => current + 1);
  }

  async function runScan(target?: DesktopScanTarget) {
    setScanState("running");
    setScanError("");
    try {
      const report = await invokeDesktop<EnforcerReport>("run_packaged_scan", { root: selectedProject.root, target });
      setReportsByProject((current) => ({ ...current, [selectedProject.id]: report }));
      const history = await invokeDesktop<DesktopScanHistoryEntry[]>("load_desktop_scan_history", { root: selectedProject.root });
      setScanHistoryByProject((current) => ({ ...current, [selectedProject.id]: history }));
      setScanState("complete");
    } catch (error) {
      setScanState("failed");
      setScanError(String(error));
    }
  }


  async function runProjectAnalysis() {
    setAnalysisLoading(true);
    setAnalysisError("");
    try {
      const nativeRun = await invokeDesktop<LegacyAnalysisPayload>("run_legacy_analysis", { root: selectedProject.root, kind: analysisKind });
      const run = analysisRunFromNative(nativeRun);
      setAnalysisRunsByProject((current) => ({ ...current, [selectedProject.id]: { ...current[selectedProject.id], [analysisKind]: run } }));
    } catch (error) {
      setAnalysisError(`Analysis unavailable: ${String(error)}`);
    } finally {
      setAnalysisLoading(false);
    }
  }

  async function refreshHarnessRuns() {
    setHarnessRunsLoading(true);
    setHarnessRunsError("");
    setHarnessRunDetail(undefined);
    setSelectedHarnessRunId("");
    try {
      setHarnessRuns(await invokeDesktop<HarnessRunPayload>("load_harness_runs", { root: selectedProject.root }));
    } catch (error) {
      setHarnessRuns(undefined);
      setHarnessRunsError(`Harness run history unavailable: ${String(error)}`);
    } finally {
      setHarnessRunsLoading(false);
    }
  }

  async function selectHarnessRun(runId: string) {
    setSelectedHarnessRunId(runId);
    setHarnessRunsError("");
    try {
      setHarnessRunDetail(await invokeDesktop<HarnessRunDetailPayload>("load_harness_run_detail", { root: selectedProject.root, runId }));
    } catch (error) {
      setHarnessRunDetail(undefined);
      setHarnessRunsError(`Harness run detail unavailable: ${String(error)}`);
    }
  }

  async function loadScanRun(runId: string) {
    setScanError("");
    try {
      const report = await invokeDesktop<EnforcerReport>("load_desktop_scan_run", { root: selectedProject.root, runId });
      setReportsByProject((current) => ({ ...current, [selectedProject.id]: report }));
      setScanState("complete");
    } catch (error) {
      setScanState("failed");
      setScanError(`Scan run unavailable: ${String(error)}`);
    }
  }

  async function runMemorySearch(query: string) {
    setMemorySearchLoading(true);
    setMemorySearchError("");
    try {
      setMemorySearch(await invokeDesktop<GraphSearchPayload>("search_memory_graph", { root: selectedProject.root, query }));
    } catch (error) {
      setMemorySearchError(String(error));
      setMemorySearch(undefined);
    } finally {
      setMemorySearchLoading(false);
    }
  }

  useEffect(() => {
    setSelectedFindingId(findings[0]?.id ?? "");
  }, [selectedProject.id, findings]);

  async function updateProjectOverride(override: RuleOverride) {
    const settings = await invokeDesktop<ProjectSettingsPayload>("write_rule_override", {
      root: selectedProject.root,
      request: {
        ruleId: override.ruleId,
        enabled: override.enabled,
        severity: override.severity ?? null,
        waiver: override.waiver ?? null,
      },
    });
    setProjectSettings(settings);
    setOverridesByProject((current) => ({ ...current, [selectedProject.id]: [] }));
  }

  async function registerProject(project: Project) {
    const registered = await invokeDesktop<Project[]>("register_desktop_project", { project });
    setProjects((current) => mergeProjects(current, registered));
  }

  async function previewProjectRegistration(root: string) {
    return invokeDesktop<ProjectRegistrationPreview>("preview_desktop_project_registration", { root });
  }

  async function discoverProjectWorktrees() {
    const discovery = await invokeDesktop<ProjectDiscoveryPayload>("discover_desktop_project_worktrees", { root: selectedProject.root });
    setProjects((current) => mergeProjects(current, discovery.projects));
  }

  async function sendHubMessage(recipientLane: string, body: string) {
    const updated = await invokeDesktop<HubPayload>("send_hub_message", {
      request: { recipientLane, body },
    });
    setHub(updated);
  }

  async function acknowledgeHubMessage(messageId: string) {
    const updated = await invokeDesktop<HubPayload>("acknowledge_hub_message", { messageId });
    setHub(updated);
  }

  async function createHubClaim(request: { projectRoot: string; laneId: string; path: string; reason: string }) {
    const updated = await invokeDesktop<HubPayload>("create_hub_claim", { request });
    setHub(updated);
  }

  async function writeScanScopeSettings(request: Pick<ScanScopeSettingsPayload, "profileName" | "ignoreDirs" | "ignoreFileGlobs">) {
    const updated = await invokeDesktop<ScanScopeSettingsPayload>("write_scan_scope_settings", { root: selectedProject.root, request });
    setScanScopeSettings(updated);
  }

  function navigateEngineCapability(target: EngineCapabilityTarget) {
    const hubView = target.workspace === "hub" ? resolveHubView(target.subview) : undefined;
    if (hubView) {
      setHubView(hubView);
    }
    setWorkspace(target.workspace);
  }

  return (
    <AppShell active={workspace} onNavigate={setWorkspace} nativeShell={nativeShell} bindingMode={bindingMode}>
      {workspace !== "projects" && workspace !== "engine" && workspace !== "hub" && (
        <CommandBar
          project={selectedProject}
          projects={projects}
          onProjectChange={setSelectedProjectId}
          nativeShell={nativeShell}
          bindingMode={bindingMode}
        />
      )}
      <main className="app-grid task-only">
        {workspace === "projects" ? (
          <ProjectsWorkspace
            projects={projects}
            registryError={projectRegistryError}
            onAddProject={registerProject}
            onPreviewProjectRegistration={previewProjectRegistration}
            onDiscoverProjectWorktrees={discoverProjectWorktrees}
            selectedProjectId={selectedProjectId}
            onOpenProject={(id) => {
              setSelectedProjectId(id);
              setWorkspace("overview");
            }}
          />
        ) : workspace === "overview" ? (
          <ProjectOverviewWorkspace project={selectedProject} report={report} catalog={ruleCatalog} onNavigate={setWorkspace} />
        ) : workspace === "setup" ? (
          <ProjectSetupWorkspace
            project={selectedProject}
            settings={projectSettings}
            settingsLoading={projectSettingsLoading}
            settingsError={projectSettingsError}
            scanScopeSettings={scanScopeSettings}
            scanScopeSettingsLoading={scanScopeSettingsLoading}
            scanScopeSettingsError={scanScopeSettingsError}
            proofSnapshot={proofSnapshot}
            proofLoading={proofArtifactsLoading}
            proofError={proofArtifactsError}
            onNavigate={(nextWorkspace, nextSettingsTab) => {
              if (nextSettingsTab) setSettingsTab(nextSettingsTab);
              setWorkspace(nextWorkspace);
            }}
          />
        ) : workspace === "engine" ? (
          <EngineWorkspace capabilities={engineCapabilities} loading={engineCapabilitiesLoading} error={engineCapabilitiesError} workpackIndex={workpackIndex} workpackIndexLoading={workpackIndexLoading} workpackIndexError={workpackIndexError} onNavigate={navigateEngineCapability} />
        ) : workspace === "analysis" ? (
          <AnalysisWorkspace kind={analysisKind} run={analysisRunsByProject[selectedProject.id]?.[analysisKind]} loading={analysisLoading} error={analysisError} onKindChange={(kind) => { setAnalysisKind(kind); setAnalysisError(""); }} onRun={runProjectAnalysis} />
        ) : workspace === "runs" ? (
          <RunsWorkspace payload={harnessRuns ? { ...harnessRuns, selectedRun: harnessRunDetail } : undefined} loading={harnessRunsLoading} error={harnessRunsError} selectedRunId={selectedHarnessRunId} onSelectRun={selectHarnessRun} onRefresh={refreshHarnessRuns} />
        ) : workspace === "rules" ? (
          <RuleCatalogWorkspace
            project={selectedProject}
            catalog={ruleCatalog}
            catalogLoading={ruleCatalogLoading}
            catalogError={ruleCatalogError}
            coverage={projectRuleCoverage}
            overrides={projectOverrides}
            focusRuleId={ruleFocusId}
            onUpdateOverride={updateProjectOverride}
          />
        ) : workspace === "doctrine" ? (
          <DoctrineWorkspace
            project={selectedProject}
            catalog={ruleCatalog}
            overrides={projectOverrides}
            settings={projectSettings}
            settingsLoading={projectSettingsLoading}
            settingsError={projectSettingsError}
            scanScopeSettings={scanScopeSettings}
            onOpenRules={() => setWorkspace("rules")}
            onOpenSettings={() => setWorkspace("settings")}
          />
        ) : workspace === "settings" ? (
          <ProjectSettingsWorkspace project={selectedProject} initialTab={settingsTab} onCreateMemoryIndex={createMemoryIndex} settings={projectSettings} settingsLoading={projectSettingsLoading} settingsError={projectSettingsError} scanScopeSettings={scanScopeSettings} scanScopeSettingsLoading={scanScopeSettingsLoading} scanScopeSettingsError={scanScopeSettingsError} onWriteScanScopeSettings={writeScanScopeSettings} />
        ) : workspace === "assurance" ? (
          <AssuranceWorkspace project={selectedProject} profile={securityProfile} loading={securityProfileLoading} error={securityProfileError} onActivate={activateSecurityProfile} />
        ) : workspace === "memory" ? (
          <MemoryExplorerWorkspace graph={graph} graphLoading={graphLoading} summary={memorySummary} summaryLoading={memorySummaryLoading} search={memorySearch} searchLoading={memorySearchLoading} searchError={memorySearchError} onSearch={runMemorySearch} onLoadSourceSnippet={loadGraphSourceSnippet} onOpenIndex={() => { setSettingsTab("index"); setWorkspace("settings"); }} onRefreshGraph={() => setMemoryRevision((current) => current + 1)} onFocusGraph={setGraphFocus} onClearGraphFocus={() => setGraphFocus(undefined)} />
        ) : workspace === "hub" ? (
          <HubWorkspace hub={hub} loading={hubLoading} error={hubError} handoff={hubFindingHandoff} initialView={hubView} harnessDiscovery={harnessDiscovery} harnessDiscoveryLoading={harnessDiscoveryLoading} harnessDiscoveryError={harnessDiscoveryError} onRefreshHarnesses={refreshHarnessDiscovery} onSendMessage={sendHubMessage} onAcknowledgeMessage={acknowledgeHubMessage} onCreateClaim={createHubClaim} onClearHandoff={() => setHubFindingHandoff(undefined)} />
        ) : workspace === "proofs" ? (
          <ProofWorkspace project={selectedProject} snapshot={proofSnapshot} artifacts={proofArtifacts} loading={proofArtifactsLoading} error={proofArtifactsError} onOpenRules={() => setWorkspace("rules")} onOpenScan={() => setWorkspace("findings")} />
        ) : (
          <FindingsWorkspace
            report={report}
            findings={findings}
            waivableRuleIds={ruleCatalog.filter((rule) => rule.waivable).map((rule) => rule.id)}
            selectedFinding={selectedFinding}
            onSelectFinding={setSelectedFindingId}
            onOpenRules={(ruleId) => { setRuleFocusId(ruleId); setWorkspace("rules"); }}
            onOpenHubHandoff={(finding) => {
              setHubFindingHandoff({ projectName: selectedProject.name, projectRoot: selectedProject.root, ruleId: finding.ruleId, title: finding.title, file: finding.file, line: finding.line, detail: finding.summary });
              setWorkspace("hub");
            }}
            onWaiveFinding={async (finding, owner, reason) => {
              const nextReport = await invokeDesktop<EnforcerReport>("waive_packaged_finding", { root: selectedProject.root, request: { path: finding.file, ruleId: finding.ruleId, owner, reason, expires: null } });
              setReportsByProject((current) => ({ ...current, [selectedProject.id]: nextReport }));
              setScanState("complete");
            }}
            onRunScan={runScan}
            scanTargets={scanTargets}
            scanTargetsLoading={scanTargetsLoading}
            scanTargetsError={scanTargetsError}
            scanHistory={scanHistoryByProject[selectedProject.id] ?? []}
            onLoadScanRun={loadScanRun}
            runState={scanState}
            scanError={scanError}
          />
        )}
      </main>
    </AppShell>
  );
}
