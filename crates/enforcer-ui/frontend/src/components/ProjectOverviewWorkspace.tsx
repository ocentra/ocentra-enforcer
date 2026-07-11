import {
  Activity,
  BookOpenCheck,
  FileCheck2,
  FileSearch,
  Network,
  ScanLine,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Wrench,
} from "lucide-react";
import type { WorkspaceKey } from "./AppShell";
import { summarizeLanguages, type Project } from "../data/enforcerAppData";
import { projectRuleLanguages, type CatalogRule, unsupportedProjectRuleLanguages } from "../data/ruleCatalog";

type ScanSummary = {
  totalCount: number;
  violations: unknown[];
  warnings: unknown[];
  runtime?: string;
  targetLabel?: string;
};

type Destination = {
  workspace: WorkspaceKey;
  title: string;
  detail: string;
  state: string;
  icon: typeof ScanLine;
  tone?: "ready" | "partial" | "missing";
};

export function ProjectOverviewWorkspace({
  project,
  report,
  catalog,
  onNavigate,
}: {
  project: Project;
  report: ScanSummary;
  catalog: CatalogRule[];
  onNavigate: (workspace: WorkspaceKey) => void;
}) {
  const scanLoaded = report.runtime === "packaged-enforcer-command";
  const policyLanguages = projectRuleLanguages(project, catalog);
  const observedWithoutPolicy = unsupportedProjectRuleLanguages(project, catalog);
  const destinations: Destination[] = [
    {
      workspace: "setup",
      title: "Setup",
      detail: "See the project lifecycle as separate registration, scope, policy, index, proof, baseline, and CI states. Global adapters live in Hub.",
      state: "readiness map",
      icon: Wrench,
      tone: "partial",
    },
    {
      workspace: "findings",
      title: "Scan",
      detail: scanLoaded ? `${report.violations.length} blocking / ${report.warnings.length} warning findings in the loaded desktop snapshot.` : "No packaged scan snapshot is loaded for this project.",
      state: scanLoaded ? `${report.totalCount} findings` : "not loaded",
      icon: ScanLine,
      tone: scanLoaded ? (report.violations.length > 0 ? "missing" : "ready") : "partial",
    },
    {
      workspace: "rules",
      title: "Rules",
      detail: `Numbered rules scoped to ${policyLanguages.join(", ")}. ${observedWithoutPolicy.length ? `${summarizeLanguages(observedWithoutPolicy)} is observed but has no policy family yet.` : ""}`,
      state: "catalog",
      icon: BookOpenCheck,
      tone: "ready",
    },
    {
      workspace: "doctrine",
      title: "Policy",
      detail: "Project-wide rule policy, scan exclusions, and native tool ties. A policy edit is not an individual finding waiver.",
      state: "project-wide",
      icon: Settings2,
      tone: "partial",
    },
    {
      workspace: "proofs",
      title: "Proofs",
      detail: "Read the Rust proof journal, proof-run artifacts, freshness, and PR-ready claim state for this root.",
      state: "read model",
      icon: FileCheck2,
      tone: "partial",
    },
    {
      workspace: "memory",
      title: "Memory",
      detail: "Open the X06 Store graph, deterministic retrieval, and parity evidence. Graph scale remains bounded by the current desktop projection.",
      state: project.indexed,
      icon: Network,
      tone: project.indexed === "ready" ? "ready" : "partial",
    },
    {
      workspace: "analysis",
      title: "Analysis",
      detail: "Run focused test-posture or UI-boundary analysis. These reports use the legacy bridge and do not create Rust-native history.",
      state: "on demand",
      icon: FileSearch,
      tone: "partial",
    },
    {
      workspace: "assurance",
      title: "Assurance",
      detail: "Inspect the Rust security profile, categories, invariants, and current activation intent for the selected project.",
      state: "profile",
      icon: ShieldCheck,
      tone: "partial",
    },
    {
      workspace: "runs",
      title: "Runs",
      detail: "Read the harness run store, diagnostics, and bounded redacted artifacts. Desktop execution controls are not implemented.",
      state: "read-only",
      icon: Activity,
      tone: "partial",
    },
  ];

  return (
    <section className="main-surface project-overview-workspace">
      <header className="overview-heading">
        <span>
          <small>Selected project</small>
          <strong>{project.name}</strong>
          <em>{project.root}</em>
        </span>
        <span className={`overview-index-state ${project.indexed}`}>
          {project.indexed === "ready" ? "index ready" : project.indexed === "stale" ? "index stale" : "index not found"}
        </span>
      </header>

      <section className="overview-facts" aria-label="Selected project facts">
        <Fact label="Topology" value={project.kind === "worktree" ? `worktree / ${project.worktree}` : `${project.kind} / ${project.worktree}`} />
        <Fact label="Branch" value={project.branch} />
        <Fact label="Observed stack" value={summarizeLanguages(project.detectedLanguages)} />
        <Fact label="Inspection" value={project.inspection ?? "configured"} />
      </section>

      <section className="overview-destinations" aria-label="Project workspaces">
        <div className="panel-head">
          <span><strong>Project workspaces</strong><small>Each route opens a distinct engine surface. States identify the current boundary, not a promised capability.</small></span>
          <SlidersHorizontal size={18} />
        </div>
        <div className="overview-destination-grid">
          {destinations.map((destination) => {
            const Icon = destination.icon;
            return (
              <button key={destination.workspace} className="overview-destination" onClick={() => onNavigate(destination.workspace)}>
                <Icon size={19} />
                <span>
                  <strong>{destination.title}</strong>
                  <small>{destination.detail}</small>
                </span>
                <em className={destination.tone}>{destination.state}</em>
              </button>
            );
          })}
        </div>
      </section>
    </section>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return <span><small>{label}</small><strong>{value}</strong></span>;
}
