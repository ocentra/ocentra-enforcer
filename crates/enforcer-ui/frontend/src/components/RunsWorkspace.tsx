import { AlertTriangle, Database, FileText, RefreshCw, TerminalSquare } from "lucide-react";
import { type ReactElement } from "react";

/** Captures a harness execution status supplied by the native run store. */
export type HarnessRunStatus = string;

/** Represents one summary row in the harness execution history. */
export type HarnessRunRow = {
  runId: string;
  tool: string;
  language?: string | null;
  command: string[];
  status: HarnessRunStatus;
  exitCode: number;
  startedAt: string;
  endedAt: string;
  diagnosticCount: number;
  pinned: boolean;
  storageRoot: string;
};

/** Represents one normalized diagnostic attached to a harness execution. */
export type HarnessDiagnosticPayload = {
  severity: string;
  ruleId: string;
  file: string;
  line: number;
  message: string;
  source?: string | null;
};

/** Holds the bounded stdout or stderr excerpt returned for a selected execution. */
export type HarnessArtifactPayload = {
  available: boolean;
  content: string;
  error?: string | null;
};

/** Contains the selected harness run with diagnostics and bounded artifacts. */
export type HarnessRunDetailPayload = {
  run: HarnessRunRow;
  diagnostics: HarnessDiagnosticPayload[];
  stdout: HarnessArtifactPayload;
  stderr: HarnessArtifactPayload;
  caveat: string;
};

/** Represents the last recorded failed harness execution. */
export type HarnessFailurePayload = {
  run: HarnessRunRow;
  diagnostics: HarnessDiagnosticPayload[];
};

/** Defines the native harness run-store payload rendered by this workspace. */
export type HarnessRunPayload = {
  root: string;
  storage: string;
  runs: HarnessRunRow[];
  lastFailure?: HarnessFailurePayload | null;
  caveat: string;
  selectedRun?: HarnessRunDetailPayload | null;
};

/** Defines run workspace state and shell callbacks. */
export type RunsWorkspaceProps = {
  payload?: HarnessRunPayload;
  loading: boolean;
  error?: string;
  selectedRunId?: string;
  onSelectRun: (runId: string) => void;
  onRefresh: () => void;
};

/** Renders read-only native harness execution history and selected-run evidence. */
export function RunsWorkspace({ payload, loading, error, selectedRunId, onSelectRun, onRefresh }: RunsWorkspaceProps): ReactElement {
  const runs = payload?.runs ?? [];
  const passed = runs.filter((run) => run.status === "passed").length;
  const failed = runs.filter((run) => run.status === "failed").length;
  const selectedRun = runs.find((run) => run.runId === selectedRunId) ?? payload?.selectedRun?.run;
  const detailCandidate = payload?.selectedRun ?? undefined;
  const selectedDetail = detailCandidate?.run.runId === selectedRun?.runId ? detailCandidate : undefined;

  return (
    <section className="main-surface engine-workspace runs-workspace">
      <div className="engine-heading">
        <span>
          <strong>Harness runs</strong>
          <small>Execution history for native tools. Scan findings and Proof evidence are tracked in their own workspaces.</small>
        </span>
        <TerminalSquare size={20} />
      </div>

      <div className="engine-toolbar">
        <Database size={16} />
        <small>Read-only execution records from the harness run store.</small>
        <button className="primary-action" onClick={onRefresh} disabled={loading}>
          <RefreshCw size={16} /> {loading ? "Refreshing" : "Refresh runs"}
        </button>
      </div>

      <div className="runs-content">
        {error && <div className="index-error">{error}</div>}
        {loading && <div className="run-status">Loading harness run history for the selected project.</div>}

        {!loading && payload && (
          <>
            <div className="engine-metrics" aria-label="Harness run summary">
              <Metric value={runs.length} label="recorded runs" />
              <Metric value={passed} label="passed" />
              <Metric value={failed} label="failed" />
              <Metric value={runs.reduce((count, run) => count + run.diagnosticCount, 0)} label="diagnostics" />
            </div>
            <div className="policy-callout">
              <Database size={17} />
              <span>{payload.caveat} Storage: <code>{payload.storage}</code> at <code>{payload.root}</code>. Retention and legacy-storage discovery are read-only in this workspace.</span>
            </div>
          </>
        )}

        {!loading && payload && runs.length === 0 && <EmptyRuns />}
        {!loading && payload && runs.length > 0 && (
          <div className="engine-layout runs-layout">
            <section className="engine-catalog" aria-label="Harness run history">
              <div className="catalog-row catalog-header"><span>Tool run</span><span>Status</span><span>Diagnostics</span></div>
              {runs.map((run) => (
                <button
                  key={run.runId}
                  className={`catalog-row run-record ${selectedRun?.runId === run.runId ? "selected" : ""}`}
                  onClick={() => onSelectRun(run.runId)}
                  aria-pressed={selectedRun?.runId === run.runId}
                >
                  <span>
                    <strong>{run.tool}</strong>
                    <small>{run.endedAt}</small>
                    <code>{run.runId}</code>
                  </span>
                  <em className={runTone(run.status)}>{run.status}</em>
                  <i>{run.diagnosticCount} / {run.language ?? "unknown"}</i>
                </button>
              ))}
            </section>
            <RunDetail run={selectedRun} detail={selectedDetail} />
          </div>
        )}
      </div>
    </section>
  );
}

function EmptyRuns() {
  return (
    <div className="empty-inspector">
      <TerminalSquare size={26} />
      <strong>No harness run records exist</strong>
      <small>Run history is separate from a project Scan and from Proof records. This project has no readable harness executions in the configured or legacy storage roots.</small>
    </div>
  );
}

function RunDetail({ run, detail }: { run?: HarnessRunRow; detail?: HarnessRunDetailPayload }) {
  if (!run) {
    return <aside className="detail-panel engine-detail"><div className="detail-heading"><TerminalSquare size={20} /><span><strong>Select an execution</strong><small>Choose a harness run to request its diagnostics and bounded artifact excerpt.</small></span></div></aside>;
  }

  const artifact = firstAvailableArtifact(detail);
  return (
    <aside className="detail-panel engine-detail">
      <div className="detail-heading"><TerminalSquare size={20} /><span><strong>{run.tool}</strong><small>{run.runId}</small></span></div>
      <dl className="engine-facts">
        <dt>Status</dt><dd><span className={`proof-status ${runTone(run.status)}`}>{run.status}</span></dd>
        <dt>Completed</dt><dd>{run.endedAt}</dd>
        <dt>Command</dt><dd><code>{run.command.join(" ") || "not recorded"}</code></dd>
        <dt>Storage</dt><dd><code>{run.storageRoot}</code></dd>
      </dl>
      {!detail && <div className="policy-callout"><FileText size={17} /><span>Run metadata is loaded. Diagnostics and artifacts have not been supplied for this selected record.</span></div>}
      {detail && <Diagnostics diagnostics={detail.diagnostics} />}
      {artifact.state === "available" && <ArtifactExcerpt artifact={artifact.value} />}
      {detail?.caveat && <div className="policy-callout"><AlertTriangle size={17} /><span>{detail.caveat}</span></div>}
    </aside>
  );
}

function Diagnostics({ diagnostics }: { diagnostics: HarnessDiagnosticPayload[] }) {
  if (diagnostics.length === 0) return <div className="proof-empty">No diagnostics were recorded for this execution.</div>;
  return (
    <section className="run-diagnostics" aria-label="Run diagnostics">
      <div className="panel-head"><span><strong>Diagnostics</strong><small>{diagnostics.length} record{diagnostics.length === 1 ? "" : "s"}</small></span></div>
      <div className="override-summary-list">
        {diagnostics.map((diagnostic, index) => <div key={`${diagnostic.file}:${diagnostic.line}:${diagnostic.ruleId}:${index}`}><strong>{diagnostic.ruleId}</strong><span>{diagnostic.severity} / {diagnostic.file}:{diagnostic.line}</span><small>{diagnostic.message}</small></div>)}
      </div>
    </section>
  );
}

function ArtifactExcerpt({ artifact }: { artifact: { label: "stdout" | "stderr"; payload: HarnessArtifactPayload } }) {
  return (
    <section className="run-artifact-excerpt" aria-label="Bounded artifact excerpt">
      <div className="panel-head"><span><strong>Bounded/redacted artifact excerpt</strong><small>{artifact.label}</small></span><FileText size={18} /></div>
      <pre>{artifact.payload.content}</pre>
      {artifact.payload.error && <small>{artifact.payload.error}</small>}
    </section>
  );
}

type ArtifactSelection =
  | { state: "available"; value: { label: "stdout" | "stderr"; payload: HarnessArtifactPayload } }
  | { state: "unavailable" };

function firstAvailableArtifact(detail?: HarnessRunDetailPayload): ArtifactSelection {
  if (!detail) return { state: "unavailable" };
  if (detail.stderr.available && detail.stderr.content) return { state: "available", value: { label: "stderr", payload: detail.stderr } };
  if (detail.stdout.available && detail.stdout.content) return { state: "available", value: { label: "stdout", payload: detail.stdout } };
  return { state: "unavailable" };
}

function Metric({ value, label }: { value: number; label: string }) {
  return <span><strong>{value}</strong><small>{label}</small></span>;
}

function runTone(status: string): "ready" | "missing" | "partial" {
  if (status === "passed") return "ready";
  if (status === "failed") return "missing";
  return "partial";
}
