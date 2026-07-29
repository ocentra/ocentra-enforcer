import { BarChart3, CheckCircle2, CircleSlash, Clock3, FileWarning, Filter, GitPullRequestArrow, History, Play, Search, SlidersHorizontal, X } from "lucide-react";
import { type ReactElement, useEffect, useMemo, useState } from "react";
import type { UiReportResponse } from "../bindings/UiReportResponse";
import type { DisplayFinding } from "../data/reportAdapter";

type ScanHistoryEntry = { runId: string; generatedAt: string; scope: string; totalCount: number; blockingCount: number; warningCount: number; waivedCount: number; runtime: string; persistence: string };
type ScanTarget = { id: string; label: string; description: string; mode: "workspace" | "crate" | "files" | "diff"; crateName?: string; files?: string[]; base?: string; head?: string };
type ScanMode = "workspace" | "crate" | "files" | "diff";

export function FindingsWorkspace({ report, findings, waivableRuleIds, selectedFinding, onSelectFinding, onOpenRules, onOpenHubHandoff, onWaiveFinding, onRunScan, scanTargets, scanTargetsLoading, scanTargetsError, scanHistory, onLoadScanRun, runState, scanError }: { report: UiReportResponse & { runtime?: string; persistence?: string; generatedAt?: string; runId?: string; targetLabel?: string }; findings: DisplayFinding[]; waivableRuleIds: string[]; selectedFinding?: DisplayFinding | undefined; onSelectFinding: (id: string) => void; onOpenRules: (ruleId?: string) => void; onOpenHubHandoff: (finding: DisplayFinding) => void; onWaiveFinding: (finding: DisplayFinding, owner: string, reason: string) => Promise<void>; onRunScan: (target?: ScanTarget) => void; scanTargets: ScanTarget[]; scanTargetsLoading: boolean; scanTargetsError: string; scanHistory: ScanHistoryEntry[]; onLoadScanRun: (runId: string) => void; runState: "idle" | "running" | "complete" | "failed"; scanError: string }): ReactElement {
  const [category, setCategory] = useState<string | null>(null);
  const [view, setView] = useState<"findings" | "priorities">("findings");
  const [query, setQuery] = useState("");
  const [severityFilter, setSeverityFilter] = useState<"all" | "error" | "warning">("all");
  const [statusFilter, setStatusFilter] = useState<"all" | DisplayFinding["status"]>("all");
  const [scopeMode, setScopeMode] = useState<ScanMode>("workspace");
  const [selectedCrateId, setSelectedCrateId] = useState("");
  const [filePaths, setFilePaths] = useState("");
  const [diffBase, setDiffBase] = useState("HEAD~1");
  const [diffHead, setDiffHead] = useState("HEAD");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [waiverOwner, setWaiverOwner] = useState("");
  const [waiverReason, setWaiverReason] = useState("");
  const [waiverBusy, setWaiverBusy] = useState(false);
  const [waiverError, setWaiverError] = useState("");
  const crateTargets = scanTargets.filter((target) => target.mode === "crate");
  const pathTargets = scanTargets.filter((target) => target.mode === "files");
  const workspaceTarget = scanTargets.find((target) => target.mode === "workspace");
  const selectedCrate = crateTargets.find((target) => target.id === selectedCrateId) ?? crateTargets[0];
  const explicitPaths = filePaths.split(/[\n,]/u).map((value) => value.trim()).filter(Boolean);
  const requestedTarget = scopeMode === "workspace" ? workspaceTarget : scopeMode === "crate" ? selectedCrate : scopeMode === "files" && explicitPaths.length ? { id: `files:${explicitPaths.join(",")}`, label: explicitPaths.length === 1 ? (explicitPaths[0] ?? "Selected path") : `${explicitPaths.length} selected paths`, description: "Explicit project-relative file or directory scan.", mode: "files" as const, files: explicitPaths } : scopeMode === "diff" && diffBase.trim() && diffHead.trim() ? { id: `diff:${diffBase}..${diffHead}`, label: `${diffBase}..${diffHead}`, description: "Files changed between two Git revisions.", mode: "diff" as const, base: diffBase.trim(), head: diffHead.trim() } : undefined;
  const scopeLabel = report.targetLabel || (report.scope === "all" ? "Entire workspace" : report.scope);
  const scanModeLabel = scopeMode === "workspace" ? "Entire project" : scopeMode === "crate" ? "Rust crate" : scopeMode === "files" ? "Folder or path" : "Git diff";
  useEffect(() => {
    if (scopeMode === "crate" && crateTargets.length === 0) setScopeMode("workspace");
    if (!selectedCrateId && crateTargets[0]) setSelectedCrateId(crateTargets[0].id);
  }, [crateTargets, scopeMode, selectedCrateId]);
  const activeFindings = useMemo(() => findings.filter((finding) => finding.status !== "waived"), [findings]);
  const waivedCount = findings.length - activeFindings.length;
  const categoryCounts = useMemo(() => findings.reduce<Array<{ category: string; active: number; blocking: number; warnings: number; waived: number }>>((acc, finding) => {
    const existing = acc.find((item) => item.category === finding.category);
    const target = existing ?? { category: finding.category, active: 0, blocking: 0, warnings: 0, waived: 0 };
    if (!existing) acc.push(target);
    if (finding.status === "waived") target.waived += 1;
    else {
      target.active += 1;
      if (finding.severity === "error") target.blocking += 1;
      if (finding.status === "warning") target.warnings += 1;
    }
    return acc;
  }, []), [findings]);
  const visibleFindings = (category ? findings.filter((finding) => finding.category === category) : findings).filter((finding) => {
    const matchesQuery = !query.trim() || `${finding.ruleId} ${finding.title} ${finding.file} ${finding.category}`.toLowerCase().includes(query.trim().toLowerCase());
    const matchesSeverity = severityFilter === "all" || finding.severity === severityFilter;
    const matchesStatus = statusFilter === "all" || finding.status === statusFilter;
    return matchesQuery && matchesSeverity && matchesStatus;
  });
  const activeFinding = visibleFindings.find((finding) => finding.id === selectedFinding?.id) ?? visibleFindings[0];
  const activeFindingCanBeWaived = activeFinding ? waivableRuleIds.includes(activeFinding.ruleId) : false;
  const priorities = useMemo(() => ({
    files: priorityGroups(activeFindings, (finding) => finding.file, (finding) => `${finding.file}:${finding.line}`),
    rules: priorityGroups(activeFindings, (finding) => finding.ruleId, (finding) => finding.title),
  }), [activeFindings]);

  return (
    <section className="main-surface scan-workspace">
      <aside className="scan-category-panel">
        <div className="panel-head"><span><strong>Finding categories</strong><small>{activeFindings.length} active / {waivedCount} waived from the current report.</small></span><Filter size={17} /></div>
        <div className="scan-category-scroll">
          <button className={category === null ? "category-break-row selected" : "category-break-row"} onClick={() => setCategory(null)}><span><strong>All findings</strong><small>{report.violations.length} blocking / {report.warnings.length} warning / {report.waived.length} waived</small></span><em>{activeFindings.length}</em></button>
          <div className="category-breakdown">{categoryCounts.map((item) => <button className={category === item.category ? "category-break-row selected" : "category-break-row"} key={item.category} onClick={() => setCategory(item.category)}><span><strong>{item.category}</strong><small>{item.blocking} blocking / {item.warnings} warning{item.waived ? ` / ${item.waived} waived` : ""}</small></span><em>{item.active}</em></button>)}</div>
          <section className="scan-history"><div className="scan-history-title"><History size={15} /><strong>Recent runs</strong></div><small>Desktop-cache snapshots from the packaged command. They are not canonical Rust Report history.</small>{scanHistory.length === 0 ? <span className="scan-history-empty">No persisted scan runs.</span> : scanHistory.map((run) => <button className={run.runId === report.runId ? "scan-history-row selected" : "scan-history-row"} key={run.runId} onClick={() => onLoadScanRun(run.runId)}><strong>{run.totalCount} findings</strong><span>{run.blockingCount} blocking / {run.warningCount} warning / {run.waivedCount} waived</span><small>{run.generatedAt}</small></button>)}</section>
        </div>
      </aside>
        <div className="scan-panel">
          <div className="panel-head scan-run-head"><span><strong>Scan report</strong><small>{report.runtime === "packaged-enforcer-command" ? "Cached report from the packaged Enforcer scanner." : "No scan run has been loaded for this project."}</small></span><div className="scan-run-actions"><div className="scan-view-tabs" role="tablist" aria-label="Scan report view"><button role="tab" aria-selected={view === "findings"} className={view === "findings" ? "active" : ""} onClick={() => setView("findings")}>Findings</button><button role="tab" aria-selected={view === "priorities"} className={view === "priorities" ? "active" : ""} onClick={() => setView("priorities")}><BarChart3 size={14} /> Prioritize</button></div></div></div>
          <div className="scan-controls">
            <div className="scan-control-row">
              <div className="scan-target-modes"><span>Scan target</span><div className="scan-scope-tabs" role="tablist" aria-label="Scan target">
                <button className={scopeMode === "workspace" ? "active" : ""} onClick={() => setScopeMode("workspace")}>Project</button>
                <button className={scopeMode === "crate" ? "active" : ""} onClick={() => setScopeMode("crate")} disabled={scanTargetsLoading || crateTargets.length === 0} title={crateTargets.length === 0 ? "No Cargo packages were discovered under the selected project root." : "Choose a Rust Cargo package to scan."}>Rust crates {crateTargets.length > 0 ? `(${crateTargets.length})` : ""}</button>
                <button className={scopeMode === "files" ? "active" : ""} onClick={() => setScopeMode("files")}>Folder or path</button>
                <button className={scopeMode === "diff" ? "active" : ""} onClick={() => setScopeMode("diff")}>Git diff</button>
              </div>
              </div>
              {scopeMode === "crate" && selectedCrate && <label className="scan-target-select" title={selectedCrate.description}><span>Cargo package</span><select value={selectedCrate.id} onChange={(event) => setSelectedCrateId(event.target.value)} aria-label="Cargo package">{crateTargets.map((target) => <option key={target.id} value={target.id}>{target.label}</option>)}</select></label>}
              {scopeMode === "files" && <><label className="scan-path-input"><span>Files or directories under this project</span><input value={filePaths} onChange={(event) => setFilePaths(event.target.value)} placeholder="src/lib.rs, crates/enforcer-memory" title="Enter one or more project-relative files or directories." /></label>{pathTargets.length > 0 && <label className="scan-target-select"><span>Discovered directory</span><select value="" onChange={(event) => { if (event.target.value) setFilePaths(event.target.value); }} aria-label="Discovered project directory"><option value="">Choose a project directory</option>{pathTargets.map((target) => <option key={target.id} value={target.files?.join(",")}>{target.label}</option>)}</select></label>}</>}
              {scopeMode === "diff" && <div className="scan-diff-inputs"><label><span>Base</span><input value={diffBase} onChange={(event) => setDiffBase(event.target.value)} /></label><label><span>Head</span><input value={diffHead} onChange={(event) => setDiffHead(event.target.value)} /></label></div>}
              <button className="primary-action" onClick={() => onRunScan(requestedTarget)} disabled={runState === "running" || !requestedTarget}><Play size={16} /> {runState === "running" ? "Scanning" : "Run scan"}</button>
            </div>
            <div className={scanTargetsError ? "scan-target-note error" : "scan-target-note"} aria-live="polite"><strong>{scanModeLabel}</strong><span>{scanTargetsLoading ? "Discovering project targets..." : scanTargetsError ? "Target discovery is unavailable for this project." : requestedTarget ? `${requestedTarget.label}: ${requestedTarget.description}` : "Choose a valid target before starting the scan."}</span><small>Loaded report: {scopeLabel}</small></div>
          </div>
        {runState === "complete" && <div className="run-status"><CheckCircle2 size={16} /> {report.runId ? `Loaded desktop scan snapshot ${report.runId}.` : "Cached workspace report loaded from the packaged scanner."} {report.persistence === "desktop-cached-packaged-report" ? "Canonical Rust Report persistence is not implemented yet." : ""}</div>}
        {runState === "failed" && <div className="index-error">{scanError}</div>}
        <div className="scan-results">
          {view === "findings" && <><div className="scan-filter-bar"><label><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter rule, file, category" /></label><select value={severityFilter} onChange={(event) => setSeverityFilter(event.target.value as typeof severityFilter)} aria-label="Filter severity"><option value="all">All severities</option><option value="error">Blocking</option><option value="warning">Warnings</option></select><select value={statusFilter} onChange={(event) => setStatusFilter(event.target.value as typeof statusFilter)} aria-label="Filter status"><option value="all">All states</option><option value="open">Open</option><option value="warning">Warnings</option><option value="waived">Waived</option></select></div><div className="finding-table"><div className="finding-header"><span>Severity</span><span>Finding</span><span>Status</span></div>{visibleFindings.map((finding) => <button key={finding.id} className={finding.id === activeFinding?.id ? "finding-row selected" : "finding-row"} onClick={() => { onSelectFinding(finding.id); setInspectorOpen(true); }}><span className={`severity ${finding.severity === "error" ? "block" : finding.severity === "warning" ? "warn" : "info"}`}>{finding.severity}</span><span><strong>{finding.title}</strong><small>{finding.ruleId} / {finding.file}:{finding.line}</small></span><em>{finding.status}</em></button>)}</div></>}
          {view === "priorities" && <PriorityView priorities={priorities} onOpenFinding={(id) => { onSelectFinding(id); setInspectorOpen(true); setView("findings"); }} />}
        </div>
      </div>
      <aside className={inspectorOpen ? "detail-panel scan-finding-inspector is-open" : "detail-panel scan-finding-inspector"}>
        {activeFinding ? <><div className="detail-heading"><FileWarning size={20} /><span><strong>{activeFinding.ruleId}</strong><small>{activeFinding.category}</small></span><button className="icon-button inspector-close" onClick={() => setInspectorOpen(false)} title="Close finding details"><X size={17} /></button></div><h2>{activeFinding.title}</h2><p>{activeFinding.summary}</p><dl className="meta-grid"><dt>Rule</dt><dd>{activeFinding.ruleId}</dd><dt>Owner</dt><dd>{activeFinding.owner}</dd><dt>File</dt><dd>{activeFinding.file}:{activeFinding.line}</dd><dt>Status</dt><dd>{activeFinding.status}</dd></dl>{activeFinding.doc && <div className="rule-doc-anchor"><small>Rule guide</small><code>{activeFinding.doc}</code></div>}{activeFinding.status === "waived" && <section className="waiver-audit"><div className="waiver-audit-head"><CircleSlash size={16} /><span><strong>Waiver audit record</strong><small>Returned by the packaged scanner from the project waiver registry.</small></span></div><dl className="meta-grid"><dt>Waiver ID</dt><dd>{activeFinding.waiverId ?? "Not supplied"}</dd><dt>Owner</dt><dd>{activeFinding.waiverOwner ?? "Not supplied"}</dd><dt>Reason</dt><dd>{activeFinding.waiverReason ?? "Not supplied"}</dd><dt>Expires</dt><dd>{activeFinding.waiverExpires ?? "No expiry"}</dd><dt>Source</dt><dd>{activeFinding.waiverSource ?? "Not supplied"}</dd></dl></section>}{activeFinding.snippet && <pre className="finding-snippet">{activeFinding.snippet}</pre>}<div className="action-row">{activeFinding.status !== "waived" && <button className="primary-action" onClick={() => onOpenHubHandoff(activeFinding)} title="Choose a Hub lane and explicitly claim this finding file"><GitPullRequestArrow size={16} /> Assign in Hub</button>}<button className="secondary-action" onClick={() => onOpenRules(activeFinding.ruleId)} title="Open project-wide policy for this rule; this does not waive the selected finding"><CircleSlash size={16} /> Inspect rule policy</button></div><div className="proof-strip"><CheckCircle2 size={17} /><span>{activeFinding.status === "waived" ? "This finding was waived by the scanner and remains visible for audit. It is not included in active priority queues; run a fresh scan to verify the current waiver registry." : activeFindingCanBeWaived ? "This rule permits an exact-path waiver, but the desktop creation action is not implemented yet. Hub assignment creates an exact-path claim only after a lane is chosen." : "This rule is immutable in the packaged policy and cannot receive a finding-level waiver. Hub assignment creates an exact-path claim only after a lane is chosen."}</span></div><FindingLifecycle status={activeFinding.status} canBeWaived={activeFindingCanBeWaived} /></> : <div className="empty-inspector"><FileWarning size={24} /><strong>No findings in this category</strong><small>Choose another category or rerun the scan.</small></div>}
      </aside>
      {activeFinding && activeFinding.status !== "waived" && activeFindingCanBeWaived && <WaiverQuickAction finding={activeFinding} owner={waiverOwner} reason={waiverReason} busy={waiverBusy} error={waiverError} onOwnerChange={setWaiverOwner} onReasonChange={setWaiverReason} onApply={async () => { setWaiverBusy(true); setWaiverError(""); try { await onWaiveFinding(activeFinding, waiverOwner, waiverReason); } catch (error) { setWaiverError(String(error)); } finally { setWaiverBusy(false); } }} />}
    </section>
  );
}

function WaiverQuickAction({ finding, owner, reason, busy, error, onOwnerChange, onReasonChange, onApply }: { finding: DisplayFinding; owner: string; reason: string; busy: boolean; error: string; onOwnerChange: (value: string) => void; onReasonChange: (value: string) => void; onApply: () => Promise<void> }) {
  return <section className="waiver-audit scan-waiver-action"><div className="waiver-audit-head"><CircleSlash size={16} /><span><strong>Waive {finding.ruleId}</strong><small>One exact-path exception for {finding.file}.</small></span></div><label><span>Owner</span><input value={owner} onChange={(event) => onOwnerChange(event.target.value)} /></label><label><span>Reason</span><input value={reason} onChange={(event) => onReasonChange(event.target.value)} /></label>{error && <div className="index-error">{error}</div>}<button className="secondary-action" disabled={busy || !owner.trim() || !reason.trim()} onClick={() => void onApply()}><CircleSlash size={16} /> {busy ? "Applying waiver" : "Apply waiver"}</button></section>;
}

function FindingLifecycle({ status, canBeWaived }: { status: DisplayFinding["status"]; canBeWaived: boolean }) {
  const waived = status === "waived";
  return <section className="finding-lifecycle" aria-label="Finding lifecycle availability"><div className="finding-lifecycle-head"><span><strong>Finding lifecycle</strong><small>Current desktop capability boundary</small></span><Clock3 size={16} /></div><div className="finding-lifecycle-list"><div className="finding-lifecycle-row live"><span><strong>Inspect rule and evidence</strong><small>Loaded report, source location, rule guide, and scan scope.</small></span><em>available</em></div>{waived ? <div className="finding-lifecycle-row live"><span><strong>Waiver state</strong><small>The scanner returned this result as waived; it remains visible for audit.</small></span><em>applied</em></div> : <div className="finding-lifecycle-row live"><span><strong>Assign ownership</strong><small>Hub creates one explicit project-relative path claim.</small></span><em>available</em></div>}<div className="finding-lifecycle-row partial"><span><strong>Change project policy</strong><small>Rule-wide policy exists; it does not waive this one finding.</small></span><em>project-wide only</em></div><div className={canBeWaived ? "finding-lifecycle-row live" : "finding-lifecycle-row blocked"}><span><strong>Waive this finding</strong><small>{canBeWaived ? "Creates one policy-validated exact-path waiver and refreshes the packaged scan." : "The packaged policy marks this rule immutable; an individual waiver is not allowed."}</small></span><em>{canBeWaived ? "available" : "not allowed"}</em></div><div className="finding-lifecycle-row planned"><span><strong>Fix, verify, close</strong><small>Expiry, revocation, proof, and report-closeout workflows are not implemented yet.</small></span><em>not implemented</em></div></div></section>;
}

type PriorityGroup = { key: string; detail: string; count: number; blocking: number; warnings: number; sampleId: string };

function priorityGroups(findings: DisplayFinding[], keyFor: (finding: DisplayFinding) => string, detailFor: (finding: DisplayFinding) => string): PriorityGroup[] {
  const groups = new Map<string, PriorityGroup>();
  for (const finding of findings) {
    const key = keyFor(finding);
    const current = groups.get(key) ?? { key, detail: detailFor(finding), count: 0, blocking: 0, warnings: 0, sampleId: finding.id };
    current.count += 1;
    if (finding.severity === "error") current.blocking += 1;
    if (finding.severity === "warning") current.warnings += 1;
    groups.set(key, current);
  }
  return [...groups.values()].sort((left, right) => right.blocking - left.blocking || right.count - left.count || left.key.localeCompare(right.key));
}

function PriorityView({ priorities, onOpenFinding }: { priorities: { files: PriorityGroup[]; rules: PriorityGroup[] }; onOpenFinding: (id: string) => void }) {
  return <div className="scan-priority-view"><div className="priority-notice"><BarChart3 size={17} /><span>Counts are derived from active findings in this loaded report. Waived results remain auditable in Findings but do not affect remediation priority.</span></div><PriorityTable title="Hot files" subtitle="Files with repeated active findings in this report" groups={priorities.files} onOpenFinding={onOpenFinding} /><PriorityTable title="Repeated rules" subtitle="Rules concentrating active findings in this report" groups={priorities.rules} onOpenFinding={onOpenFinding} /></div>;
}

function PriorityTable({ title, subtitle, groups, onOpenFinding }: { title: string; subtitle: string; groups: PriorityGroup[]; onOpenFinding: (id: string) => void }) {
  return <section className="priority-table"><header><span><strong>{title}</strong><small>{subtitle}</small></span><small>{groups.length} groups</small></header>{groups.length === 0 ? <p>No findings in this report.</p> : groups.map((group) => <button key={group.key} onClick={() => onOpenFinding(group.sampleId)}><span><strong>{group.key}</strong><small>{group.detail}</small></span><em>{group.blocking} blocking</em><b>{group.count} total</b></button>)}</section>;
}
