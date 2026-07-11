import { AlertTriangle, CircleAlert, FileSearch, Play, ShieldAlert, SlidersHorizontal } from "lucide-react";
import { type ReactElement, useMemo, useState } from "react";

/** Identifies the independent project-analysis report to render. */
export type AnalysisRunKind = "test-doctrine" | "ui-logic-coupling";

type AnalysisRunMetadata = {
  root: string;
  caveat: string;
  generatedAt?: string;
  runtime?: string;
  persistence?: string;
};

/** Describes the test posture recorded for one analysis category. */
export type TestDoctrineCategory = {
  label: string;
  present: boolean;
  relevant: boolean;
  evidence: string[];
  ci: TestDoctrineCiStatus;
  ciIncludingUntracked?: TestDoctrineCiStatus | null;
};

/** Captures CI enforcement evidence for a test posture category. */
export type TestDoctrineCiStatus = {
  wired: boolean;
  blocking: boolean | null;
  evidence: Array<{ step: string; blocking: boolean }>;
};

/** Represents the complete result of a test-doctrine analysis run. */
export type TestDoctrineAnalysisRun = AnalysisRunMetadata & {
  kind: "test-doctrine";
  detected: Record<string, TestDoctrineCategory>;
  missing: Array<{ category: string; label: string; tier: "core" | "suggested" | "optional"; reason: string }>;
  ciConfigFilesFound: Array<{ path: string; tracked: boolean }>;
  hasUntrackedCiFiles: boolean;
  ciGaps: Array<{ category: string; label: string; reason: string; ciEvidence: Array<{ step: string; blocking: boolean }> }>;
  summary: {
    categoriesRelevant: number;
    categoriesPresent: number;
    categoriesMissing: number;
    coreMissing: number;
    ciGaps: number;
  };
};

/** Identifies one UI-to-logic boundary signal reported by the legacy analysis bridge. */
export type UiLogicCouplingFinding = {
  file: string;
  kind: string;
  severity: "hard" | "info";
  source: string;
  binding: string;
  hasDataFetchPrimitive?: boolean;
};

/** Represents the complete result of a UI-to-logic coupling analysis run. */
export type UiLogicCouplingAnalysisRun = AnalysisRunMetadata & {
  kind: "ui-logic-coupling";
  rule: { id: string; title: string; doc: string; aka: string; why: string };
  findings: UiLogicCouplingFinding[];
  hard: UiLogicCouplingFinding[];
  info: UiLogicCouplingFinding[];
  summary: { totalFindings: number; hardFindings: number; infoFindings: number; filesWithHardFindings: number };
};

/** Unifies the report shapes rendered by the analysis workspace. */
export type AnalysisRun = TestDoctrineAnalysisRun | UiLogicCouplingAnalysisRun;

/** Defines analysis workspace state supplied by the desktop shell. */
export type AnalysisWorkspaceProps = {
  kind: AnalysisRunKind;
  run?: AnalysisRun;
  loading: boolean;
  error?: string;
  onKindChange: (kind: AnalysisRunKind) => void;
  onRun: () => void;
};

const analysisRunKinds: readonly AnalysisRunKind[] = ["test-doctrine", "ui-logic-coupling"];

const kindCopy: Record<AnalysisRunKind, { label: string; description: string }> = {
  "test-doctrine": {
    label: "Test doctrine",
    description: "Coverage posture and CI gate evidence inferred from project signals.",
  },
  "ui-logic-coupling": {
    label: "UI boundaries",
    description: "ARCH-1.16 evidence for presentation files that may call business logic directly.",
  },
};

/** Renders focused analysis reports separately from ordinary scan findings. */
export function AnalysisWorkspace({ kind, run, loading, error, onKindChange, onRun }: AnalysisWorkspaceProps): ReactElement {
  const selectedRun = run?.kind === kind ? run : undefined;

  return (
    <section className="main-surface engine-workspace analysis-workspace">
      <div className="engine-heading">
        <span>
          <strong>Project analysis</strong>
          <small>Separate heuristic reports for test posture and UI architecture. This is not the generic Enforcer Scan.</small>
        </span>
        <FileSearch size={20} />
      </div>
      <div className="engine-toolbar">
        <SlidersHorizontal size={16} />
        <div className="segmented-control" role="tablist" aria-label="Analysis kind">
          {analysisRunKinds.map((item) => (
            <button key={item} role="tab" aria-selected={kind === item} className={kind === item ? "active" : ""} onClick={() => onKindChange(item)}>
              {kindCopy[item].label}
            </button>
          ))}
        </div>
        <small>{kindCopy[kind].description}</small>
        <button className="primary-action" onClick={onRun} disabled={loading}>
          <Play size={16} /> {loading ? "Running" : "Run analysis"}
        </button>
      </div>
      <div className="policy-callout">
        <CircleAlert size={17} />
        <span>Runs use the legacy Node analysis bridge. Rust-native analysis persistence, run history, and CI-grade execution envelopes are not implemented yet.</span>
      </div>
      <div className="analysis-content">
        {error && <div className="index-error">{error}</div>}
        {loading && <div className="run-status">Running {kindCopy[kind].label.toLowerCase()} against the selected project.</div>}
        {!loading && !selectedRun && <EmptyAnalysis kind={kind} />}
        {selectedRun?.kind === "test-doctrine" && <TestDoctrineReport run={selectedRun} />}
        {selectedRun?.kind === "ui-logic-coupling" && <UiLogicCouplingReport run={selectedRun} />}
      </div>
    </section>
  );
}

function EmptyAnalysis({ kind }: { kind: AnalysisRunKind }) {
  return (
    <div className="empty-inspector">
      <FileSearch size={26} />
      <strong>No {kindCopy[kind].label.toLowerCase()} run loaded</strong>
      <small>Run this focused analysis for the selected project. Results are not persisted as a Rust-native run history yet.</small>
    </div>
  );
}

function TestDoctrineReport({ run }: { run: TestDoctrineAnalysisRun }) {
  const relevantCategories = useMemo(
    () => Object.entries(run.detected).filter(([, category]) => category.relevant),
    [run.detected],
  );

  return (
    <>
      <div className="engine-metrics" aria-label="Test doctrine summary">
        <Metric value={run.summary.categoriesRelevant} label="relevant categories" />
        <Metric value={run.summary.categoriesPresent} label="detected locally" />
        <Metric value={run.summary.categoriesMissing} label="coverage gaps" />
        <Metric value={run.summary.ciGaps} label="CI gate gaps" />
      </div>
      <div className="settings-root-row"><span>Analysis scope</span><code title={run.root}>{run.root}</code></div>
      <div className="settings-root-row"><span>CI configuration files seen</span><code>{run.ciConfigFilesFound.length}</code></div>
      {run.hasUntrackedCiFiles && <div className="policy-callout"><AlertTriangle size={17} /><span>Untracked CI files were observed and are not credited as committed gates.</span></div>}
      {run.summary.coreMissing > 0 && <div className="policy-callout"><ShieldAlert size={17} /><span>{run.summary.coreMissing} core test posture gap{run.summary.coreMissing === 1 ? "" : "s"} need review. This is signal evidence, not a certification failure.</span></div>}
      <div className="engine-layout analysis-doctrine-layout">
        <section className="engine-catalog" aria-label="Relevant test categories">
          <div className="catalog-row catalog-header"><span>Category</span><span>Local</span><span>CI</span></div>
          {relevantCategories.map(([id, category]) => <DoctrineCategoryRow key={id} id={id} category={category} />)}
        </section>
        <aside className="detail-panel engine-detail">
          <div className="detail-heading"><ShieldAlert size={20} /><span><strong>Coverage gaps</strong><small>{run.missing.length} relevant categories were not detected</small></span></div>
          <DoctrineGapList run={run} />
        </aside>
      </div>
      <section className="scan-panel">
        <div className="panel-head"><span><strong>CI enforcement gaps</strong><small>Detected local practices that are absent from committed CI or do not block a merge.</small></span><AlertTriangle size={18} /></div>
        {run.ciGaps.length === 0 ? <div className="proof-empty">No CI enforcement gaps were reported by this analysis run.</div> : <div className="policy-table">{run.ciGaps.map((gap) => <div key={gap.category}><strong>{gap.label}</strong><span>{gap.reason}{gap.ciEvidence.length > 0 && <small>{gap.ciEvidence.map((evidence) => evidence.step).join("; ")}</small>}</span><em>review</em></div>)}</div>}
      </section>
      <AnalysisCaveat caveat={run.caveat} />
    </>
  );
}

function DoctrineCategoryRow({ id, category }: { id: string; category: TestDoctrineCategory }) {
  const ciLabel = !category.present ? "not applicable" : category.ci.blocking ? "blocking" : category.ci.wired ? "non-blocking" : "not wired";
  return <div className="catalog-row"><span><strong>{category.label}</strong><small>{id}</small>{category.evidence.length > 0 && <small>{category.evidence.join("; ")}</small>}</span><em className={category.present ? "connected" : "attention"}>{category.present ? "detected" : "missing"}</em><i className={category.ci.blocking ? "connected" : "attention"}>{ciLabel}</i></div>;
}

function DoctrineGapList({ run }: { run: TestDoctrineAnalysisRun }) {
  if (run.missing.length === 0) return <div className="proof-empty">No relevant test category was reported missing.</div>;
  return <div className="override-summary-list">{run.missing.map((gap) => <div key={gap.category}><strong>{gap.label}</strong><span>{gap.tier} priority</span><small>{gap.reason}</small></div>)}</div>;
}

function UiLogicCouplingReport({ run }: { run: UiLogicCouplingAnalysisRun }) {
  const [filter, setFilter] = useState<"all" | "hard" | "info">("all");
  const findings = useMemo(
    () => run.findings.filter((finding) => filter === "all" || finding.severity === filter),
    [filter, run.findings],
  );

  return (
    <>
      <div className="engine-metrics" aria-label="UI boundary summary">
        <Metric value={run.summary.totalFindings} label="reported signals" />
        <Metric value={run.summary.hardFindings} label="hard review signals" />
        <Metric value={run.summary.infoFindings} label="informational signals" />
        <Metric value={run.summary.filesWithHardFindings} label="files with hard signals" />
      </div>
      <div className="engine-layout">
        <section className="scan-panel">
          <div className="panel-head"><span><strong>{run.rule.id}: {run.rule.title}</strong><small><code>{run.rule.doc}</code></small></span><ShieldAlert size={18} /></div>
          <dl className="engine-facts"><dt>Why this boundary exists</dt><dd>{run.rule.why}</dd><dt>Architecture reference</dt><dd>{run.rule.aka}</dd></dl>
          <div className="settings-root-row"><span>Analysis scope</span><code>{run.root}</code></div>
        </section>
        <aside className="detail-panel engine-detail">
          <div className="detail-heading"><AlertTriangle size={20} /><span><strong>Review posture</strong><small>Findings are a first-pass boundary signal</small></span></div>
          <div className="policy-callout"><CircleAlert size={17} /><span>{run.caveat}</span></div>
        </aside>
      </div>
      <section className="scan-panel">
        <div className="panel-head"><span><strong>Boundary evidence</strong><small>Each row is an import-and-call signal to review; it is not an automatic architecture verdict.</small></span><div className="segmented-control" role="tablist" aria-label="Boundary evidence severity"><button role="tab" aria-selected={filter === "all"} className={filter === "all" ? "active" : ""} onClick={() => setFilter("all")}>All</button><button role="tab" aria-selected={filter === "hard"} className={filter === "hard" ? "active" : ""} onClick={() => setFilter("hard")}>Hard</button><button role="tab" aria-selected={filter === "info"} className={filter === "info" ? "active" : ""} onClick={() => setFilter("info")}>Info</button></div></div>
        {findings.length === 0 ? <div className="proof-empty">No {filter === "all" ? "boundary" : filter} signals were reported by this run.</div> : <div className="finding-table"><div className="finding-header"><span>Severity</span><span>Presentation boundary evidence</span><span>Signal</span></div>{findings.map((finding, index) => <div className="finding-row" key={`${finding.file}:${finding.binding}:${finding.source}:${index}`}><span className={`severity ${finding.severity === "hard" ? "block" : "info"}`}>{finding.severity}</span><span><strong>{finding.file}</strong><small>{finding.kind} / import {finding.binding} from {finding.source}</small></span><em>{finding.hasDataFetchPrimitive ? "inline data primitive" : "direct import"}</em></div>)}</div>}
      </section>
    </>
  );
}

function Metric({ value, label }: { value: number; label: string }) {
  return <span><strong>{value}</strong><small>{label}</small></span>;
}

function AnalysisCaveat({ caveat }: { caveat: string }) {
  return <div className="policy-callout"><CircleAlert size={17} /><span>{caveat}</span></div>;
}
