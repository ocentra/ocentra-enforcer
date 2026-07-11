import { FileCheck2, ShieldAlert, TerminalSquare } from "lucide-react";
import type { Project } from "../data/enforcerAppData";

type ProofArtifact = { path: string; modifiedAt: string; bytes: number };
type ProjectProofSnapshot = {
  proofRoot: string;
  currentGit: { commit?: string; branch?: string; dirty?: boolean };
  journal: { path: string; state: "missing" | "verified" | "invalid"; recordCount: number; latestEventType?: string; latestProofId?: string; latestTimestamp?: string; error?: string };
  runs: Array<{ path: string; proofRun?: { proofId: string; runId: string; title: string; capability: string; status: string; endedAt: string; pinned: boolean; diagnosticCount: number }; freshness: string; artifacts: { declared: number; present: number; missing: number; totalBytes: number }; parseError?: string }>;
  claim: { registryPath: string; state: string; requiredProofIds: string[]; claim?: { accepted: Array<{ proofId: string; runId: string }>; violations: Array<{ proofId: string; code: string; message: string }> }; error?: string };
};

function proofTone(value: string): "ready" | "partial" | "missing" {
  if (["verified", "ready", "passed", "current"].includes(value)) return "ready";
  if (["missing", "invalid", "blocked", "failed", "manual-required", "unavailable", "stale"].includes(value)) return "missing";
  return "partial";
}

export function ProofWorkspace({ project, snapshot, artifacts, loading, error, onOpenRules, onOpenScan }: { project: Project; snapshot?: ProjectProofSnapshot; artifacts: ProofArtifact[]; loading: boolean; error: string; onOpenRules: () => void; onOpenScan: () => void }) {
  const journal = snapshot?.journal;
  const claim = snapshot?.claim;
  return (
    <section className="main-surface proof-ledger-workspace">
      <div className="proof-ledger-panel">
        <div className="panel-head"><span><strong>Proof ledger</strong><small>Evidence and claim state for {project.name}.</small></span></div>
        {loading && <div className="run-status">Discovering project proof artifacts.</div>}
        {error && <div className="index-error">{error}</div>}
        <div className="proof-table">
          {journal && <div className="proof-ledger-row"><span className={`proof-status ${proofTone(journal.state)}`}>{journal.state}</span><span><strong>Hash-chained journal</strong><small>{journal.state === "verified" ? `${journal.recordCount} verified record${journal.recordCount === 1 ? "" : "s"}; latest ${journal.latestEventType ?? "event"}.` : journal.state === "invalid" ? journal.error : "No Rust proof journal has been recorded."}</small></span><code>{journal.path}</code><span>Rust proof</span></div>}
          {claim && <div className="proof-ledger-row"><span className={`proof-status ${proofTone(claim.state)}`}>{claim.state}</span><span><strong>PR-ready claim</strong><small>{claim.state === "unconfigured" ? "No project-local proofs.json registry; no claim was inferred." : claim.state === "blocked" ? `${claim.claim?.violations.length ?? 0} real claim violation(s).` : claim.state === "ready" ? `${claim.claim?.accepted.length ?? 0} required proof(s) accepted.` : claim.error ?? "No proof is required for this project."}</small></span><code>{claim.registryPath}</code><span>{claim.requiredProofIds.length} required</span></div>}
          {(snapshot?.runs ?? []).map((run) => {
            const proof = run.proofRun;
            const state = proof?.status ?? "invalid";
            return <div className="proof-ledger-row" key={run.path}><span className={`proof-status ${proofTone(state)}`}>{state}</span><span><strong>{proof?.title ?? "Invalid proof run"}</strong><small>{proof ? `${proof.proofId} / ${proof.endedAt} / ${run.freshness} commit` : run.parseError}</small></span><code>{run.path}</code><span>{run.artifacts.present}/{run.artifacts.declared} artifacts</span></div>;
          })}
          {snapshot && snapshot.runs.length === 0 && <div className="proof-empty">No Rust proof-run records exist under `{snapshot.proofRoot}/runs`.</div>}
          {artifacts.length > 0 && <div className="proof-section-label">Legacy or external files under `proof/`</div>}
          {artifacts.map((artifact) => <div className="proof-ledger-row" key={artifact.path}><span className="proof-status partial">external</span><span><strong>{artifact.path.split("/").at(-1)}</strong><small>{artifact.modifiedAt} / {artifact.bytes.toLocaleString()} bytes</small></span><code>{artifact.path}</code><span>not claim evidence</span></div>)}
        </div>
      </div>
      <aside className="proof-detail-panel">
        <div className="detail-heading"><FileCheck2 size={19} /><strong>Proof integration</strong></div>
        <div className="proof-detail-title"><span className={`proof-status ${proofTone(claim?.state ?? "missing")}`}>{claim?.state ?? "loading"}</span><h2>Project proof records</h2></div>
        <p className="detail-copy">Rust owns the proof layout, journal verification, run parsing, artifact presence, and configured PR-ready claim evaluation. A scan result never becomes proof evidence here.</p>
        <dl className="meta-grid"><dt>Journal</dt><dd>{journal?.state ?? "loading"}</dd><dt>Runs</dt><dd>{snapshot?.runs.length ?? 0} recorded</dd><dt>Git</dt><dd>{snapshot?.currentGit.commit?.slice(0, 12) ?? "unavailable"}{snapshot?.currentGit.dirty ? " / dirty" : ""}</dd><dt>Claim</dt><dd>{claim?.state ?? "loading"}</dd></dl>
        <div className="proof-command"><TerminalSquare size={16} /><code>{journal?.path ?? ".enforce/proofs/journal.ndjson"}</code></div>
        {claim?.state === "blocked" && <div className="proof-gap"><ShieldAlert size={17} /><span>{claim.claim?.violations[0]?.message ?? "The configured proof claim is blocked."}</span></div>}
        {journal?.state === "invalid" && <div className="proof-gap"><ShieldAlert size={17} /><span>Journal verification failed. The desktop refuses to treat its records as evidence.</span></div>}
        <div className="action-row proof-actions"><button className="ghost-button" onClick={onOpenScan}>Open scan</button><button className="primary-action" onClick={onOpenRules}>Inspect proof rules</button></div>
      </aside>
    </section>
  );
}
