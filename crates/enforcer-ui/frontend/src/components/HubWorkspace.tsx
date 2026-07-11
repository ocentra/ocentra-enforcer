import { BriefcaseBusiness, Cable, CheckCheck, ClipboardList, GitBranch, Inbox, RefreshCw, Search, Send, ShieldCheck, UsersRound } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

export type HubView = "lanes" | "inbox" | "claims" | "tasks" | "workers" | "harnesses";
type HubPayload = {
  rootPath: string;
  lanes: Array<{ laneId: string; writers: string[]; statusSummary?: string; heartbeatSummary?: string }>;
  claims: Array<{ claimId: string; laneId: string; writer: string; paths: string[]; reason?: string }>;
  workers: Array<{ writer: string; laneId: string; state?: string; summary?: string; currentTaskId?: string; lastSeenAt: string }>;
  tasks: Array<{ taskId: string; laneId: string; writer: string; state: string; summary: string; updatedAt: string; title?: string; prUrl?: string }>;
  mail: Array<{ messageId: string; fromWriter: string; to?: string; body?: string; ts: string; ackedBy: string[] }>;
  sync: { totalEvents: number; duplicateCount: number; warnings: string[] };
};

export type HubFindingHandoff = {
  projectName: string;
  projectRoot: string;
  ruleId: string;
  title: string;
  file: string;
  line: number;
  detail: string;
};

type HarnessEvidence = { source: string; observation: string };
type HarnessValue = { value: unknown; evidence: HarnessEvidence[] };
export type HarnessDiscoveryPayload = {
  harnesses: Array<{ id: string; present: boolean; homePath?: string; evidence: HarnessEvidence[]; capabilities?: { maxConcurrentAgents: HarnessValue; subAgentNestingDepth: HarnessValue; backgroundTasks: HarnessValue; scheduledTasks: HarnessValue; crossSessionMessaging: HarnessValue; implicitInvocation: HarnessValue } }>;
  runtime: string;
  verification: string;
};

const views: Array<{ id: HubView; label: string; icon: typeof GitBranch }> = [
  { id: "lanes", label: "Lanes", icon: GitBranch },
  { id: "inbox", label: "Inbox", icon: Inbox },
  { id: "claims", label: "Claims", icon: ShieldCheck },
  { id: "tasks", label: "Tasks", icon: ClipboardList },
  { id: "workers", label: "Workers", icon: UsersRound },
  { id: "harnesses", label: "Adapters", icon: Cable },
];

export function HubWorkspace({ hub, loading, error, handoff, initialView, harnessDiscovery, harnessDiscoveryLoading, harnessDiscoveryError, onRefreshHarnesses, onSendMessage, onAcknowledgeMessage, onCreateClaim, onClearHandoff }: { hub?: HubPayload; loading: boolean; error: string; handoff?: HubFindingHandoff; initialView: HubView; harnessDiscovery?: HarnessDiscoveryPayload; harnessDiscoveryLoading: boolean; harnessDiscoveryError: string; onRefreshHarnesses: () => Promise<void>; onSendMessage: (recipientLane: string, body: string) => Promise<void>; onAcknowledgeMessage: (messageId: string) => Promise<void>; onCreateClaim: (request: { projectRoot: string; laneId: string; path: string; reason: string }) => Promise<void>; onClearHandoff: () => void }) {
  const [view, setView] = useState<HubView>(initialView);
  const [selectedMail, setSelectedMail] = useState("");
  const [query, setQuery] = useState("");
  const filtered = useMemo(() => filterHubPayload(hub, query), [hub, query]);
  const selectedMessage = filtered.mail.find((mail) => mail.messageId === selectedMail) ?? filtered.mail[0];
  const laneStates = useMemo(() => new Map((hub?.lanes ?? []).map((lane) => [lane.laneId, lane.statusSummary ?? lane.heartbeatSummary ?? "observed"])), [hub]);
  useEffect(() => { setView(initialView); }, [initialView]);

  return (
    <section className="main-surface hub-command-center">
      <div className="scan-panel hub-main-panel">
        <div className="panel-head"><span><strong>{view === "harnesses" ? "Harness adapters" : "Lane hub"}</strong><small>{view === "harnesses" ? "User-level harness discovery and capability evidence shared across projects and lanes." : "Cross-project coordination: lanes, inbox, claims, tasks, workers, and worktrees."}</small></span>{view === "harnesses" ? <button className="primary-action" onClick={() => void onRefreshHarnesses()} disabled={harnessDiscoveryLoading}><RefreshCw size={16} /> {harnessDiscoveryLoading ? "Inspecting" : "Refresh adapters"}</button> : <BriefcaseBusiness size={18} />}</div>
        {view !== "harnesses" && <div className="hub-context-strip" aria-label="Coordination ledger context">
          <div className="hub-context-row selected"><span><strong>Ledger root</strong><small>{hub?.rootPath ?? "Loading ledger view"}</small></span><em>{hub ? "live" : "loading"}</em></div>
          <div className="hub-context-row"><span><strong>Folded events</strong><small>{hub?.sync.totalEvents ?? 0} read from coordination streams</small></span><em>{hub ? "live" : "--"}</em></div>
          <div className="hub-context-row"><span><strong>Deduplicated</strong><small>{hub?.sync.duplicateCount ?? 0} duplicate events ignored</small></span><em>{hub ? "live" : "--"}</em></div>
        </div>}
        {loading && <div className="run-status">Loading typed coordination ledger view.</div>}
        {error && <div className="index-error">{error}</div>}
          {hub && view !== "harnesses" && <div className="run-status">Live ledger: messages, acknowledgements, and explicit exact-path claims use Rust append-only events. Lane creation and automated code-fix dispatch remain unavailable.</div>}
        {hub?.sync.warnings.map((warning) => <div className="policy-callout" key={warning}><ShieldCheck size={16} /><span>{warning}</span></div>)}
        <div className="hub-tabs" role="tablist">{views.map((item) => { const Icon = item.icon; return <button key={item.id} className={view === item.id ? "hub-tab active" : "hub-tab"} onClick={() => setView(item.id)}><Icon size={15} /> {item.label}</button>; })}<label className="hub-filter"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder={view === "harnesses" ? "Search adapters" : `Filter ${view}`} /></label></div>
        {hub && view === "lanes" && <LaneList lanes={filtered.lanes} claims={filtered.claims} />}
        {hub && view === "inbox" && <InboxList inbox={filtered.mail} selected={selectedMessage?.messageId ?? ""} onSelect={setSelectedMail} />}
        {hub && view === "claims" && <ClaimList claims={filtered.claims} />}
        {hub && view === "tasks" && <TaskList tasks={filtered.tasks} />}
        {hub && view === "workers" && <WorkerList workers={filtered.workers} />}
        {view === "harnesses" && <HarnessList discovery={harnessDiscovery} loading={harnessDiscoveryLoading} error={harnessDiscoveryError} query={query} />}
        {!hub && !loading && !error && view !== "harnesses" && <div className="proof-empty">No coordination ledger is connected.</div>}
      </div>
      <aside className="detail-panel">
        {handoff ? <FindingClaimDetail handoff={handoff} lanes={hub?.lanes ?? []} onCreateClaim={onCreateClaim} onClear={onClearHandoff} /> : view === "inbox" ? <InboxDetail message={selectedMessage} lanes={hub?.lanes ?? []} onSendMessage={onSendMessage} onAcknowledgeMessage={onAcknowledgeMessage} /> : view === "harnesses" ? <HarnessDetail discovery={harnessDiscovery} /> : <HubDetail view={view} hub={hub} laneStates={laneStates} />}
      </aside>
    </section>
  );
}

function filterHubPayload(hub: HubPayload | undefined, query: string): Pick<HubPayload, "lanes" | "claims" | "workers" | "tasks" | "mail"> {
  const normalized = query.trim().toLowerCase();
  if (!hub || !normalized) return { lanes: hub?.lanes ?? [], claims: hub?.claims ?? [], workers: hub?.workers ?? [], tasks: hub?.tasks ?? [], mail: hub?.mail ?? [] };
  const includes = (...values: Array<string | undefined>) => values.filter(Boolean).join(" ").toLowerCase().includes(normalized);
  const claims = hub.claims.filter((claim) => includes(claim.claimId, claim.laneId, claim.writer, claim.reason, claim.paths.join(" ")));
  return {
    lanes: hub.lanes.filter((lane) => includes(lane.laneId, lane.writers.join(" "), lane.statusSummary, lane.heartbeatSummary) || claims.some((claim) => claim.laneId === lane.laneId)),
    claims,
    workers: hub.workers.filter((worker) => includes(worker.writer, worker.laneId, worker.state, worker.summary, worker.currentTaskId, worker.lastSeenAt)),
    tasks: hub.tasks.filter((task) => includes(task.taskId, task.title, task.laneId, task.writer, task.state, task.summary, task.updatedAt, task.prUrl)),
    mail: hub.mail.filter((mail) => includes(mail.messageId, mail.fromWriter, mail.to, mail.body, mail.ts, mail.ackedBy.join(" "))),
  };
}

function FindingClaimDetail({ handoff, lanes, onCreateClaim, onClear }: { handoff: HubFindingHandoff; lanes: HubPayload["lanes"]; onCreateClaim: (request: { projectRoot: string; laneId: string; path: string; reason: string }) => Promise<void>; onClear: () => void }) {
  const [laneId, setLaneId] = useState("");
  const [reason, setReason] = useState(`Fix ${handoff.ruleId}: ${handoff.title} at ${handoff.file}:${handoff.line}`);
  const [dispatching, setDispatching] = useState(false);
  const [result, setResult] = useState("");
  const [actionError, setActionError] = useState("");
  useEffect(() => { if (!laneId && lanes[0]) setLaneId(lanes[0].laneId); }, [laneId, lanes]);
  async function createClaim() {
    setDispatching(true);
    setActionError("");
    setResult("");
    try {
      await onCreateClaim({ projectRoot: handoff.projectRoot, laneId, path: handoff.file, reason });
      setResult(`Claim created for ${handoff.file}.`);
    } catch (error) {
      setActionError(String(error));
    } finally {
      setDispatching(false);
    }
  }
  return <><div className="detail-heading"><GitBranch size={20} /><span><strong>Scan handoff</strong><small>{handoff.projectName} / {handoff.ruleId}</small></span></div><h2>{handoff.title}</h2><p>{handoff.detail}</p><dl className="meta-grid"><dt>Finding file</dt><dd>{handoff.file}:{handoff.line}</dd><dt>Effect</dt><dd>exact-path Hub claim</dd><dt>Code edit</dt><dd>not performed</dd><dt>Proof</dt><dd>not created</dd></dl><label className="hub-composer"><span>Assign lane</span><select aria-label="Claim lane" value={laneId} onChange={(event) => setLaneId(event.target.value)} disabled={dispatching || lanes.length === 0}>{lanes.map((lane) => <option key={lane.laneId} value={lane.laneId}>{lane.laneId}</option>)}</select><span>Claim reason</span><textarea value={reason} onChange={(event) => setReason(event.target.value)} disabled={dispatching || lanes.length === 0} /><button className="primary-action" onClick={createClaim} disabled={dispatching || !laneId || !reason.trim()}><ShieldCheck size={16} /> {dispatching ? "Creating claim..." : "Create work claim"}</button></label>{lanes.length === 0 && <div className="index-error">No live lane is available. The finding remains unassigned.</div>}{result && <div className="run-status">{result}</div>}{actionError && <div className="index-error">{actionError}</div>}<button className="ghost-button full-width" onClick={onClear}>Close handoff</button></>;
}

function LaneList({ lanes, claims }: { lanes: HubPayload["lanes"]; claims: HubPayload["claims"] }) { return <div className="hub-table"><div className="hub-table-header"><span>Lane</span><span>State</span><span>Claims</span><span>Writers</span></div>{lanes.map((lane) => <div className="hub-row" key={lane.laneId}><strong>{lane.laneId}</strong><span className="hub-state active">{lane.statusSummary ?? lane.heartbeatSummary ?? "observed"}</span><span>{claims.filter((claim) => claim.laneId === lane.laneId).length} held</span><small>{lane.writers.join(", ") || "no writer observed"}</small></div>)}{lanes.length === 0 && <div className="proof-empty">No lanes were found in this ledger.</div>}</div>; }
function InboxList({ inbox, selected, onSelect }: { inbox: HubPayload["mail"]; selected: string; onSelect: (id: string) => void }) { return <div className="inbox-list">{inbox.map((mail) => <button className={mail.messageId === selected ? "mail-row selected" : "mail-row"} key={mail.messageId} onClick={() => onSelect(mail.messageId)}><strong>{mail.body ?? mail.messageId}</strong><small>{mail.fromWriter} to {mail.to ?? "broadcast"} / {mail.ts}</small></button>)}{inbox.length === 0 && <div className="proof-empty">No messages were found in this ledger.</div>}</div>; }
function ClaimList({ claims }: { claims: HubPayload["claims"] }) { return <div className="claim-list">{claims.flatMap((claim) => claim.paths.map((path) => <div className="claim-row" key={`${claim.claimId}-${path}`}><div className="claim-path"><strong title={path}>{path}</strong><span title={claim.laneId}>{claim.laneId}</span></div><div className="claim-context"><span title={claim.writer}>{claim.writer}</span><em title={claim.reason ?? claim.claimId}>{claim.reason ?? claim.claimId}</em></div></div>))}{claims.length === 0 && <div className="proof-empty">No active exact-path claims were found.</div>}</div>; }
function TaskList({ tasks }: { tasks: HubPayload["tasks"] }) { return <div className="task-list">{tasks.map((task) => <article className="task-row" key={task.taskId}><span className={`hub-state ${task.state === "done" ? "done" : task.state === "blocked" ? "blocked" : "active"}`}>{task.state}</span><strong>{task.title ?? task.taskId}</strong><small>{task.laneId} / {task.writer} / {task.updatedAt}</small><p>{task.summary}</p>{task.prUrl && <code>{task.prUrl}</code>}</article>)}{tasks.length === 0 && <div className="proof-empty">No task updates were found in this ledger.</div>}</div>; }
function WorkerList({ workers }: { workers: HubPayload["workers"] }) { return <div className="worker-list">{workers.map((worker) => <div className="worker-row" key={worker.writer}><strong>{worker.writer}</strong><span>{worker.laneId} / {worker.currentTaskId ?? "no active task"}</span><em>{worker.state ?? "observed"}</em><small>{worker.summary ?? worker.lastSeenAt}</small></div>)}{workers.length === 0 && <div className="proof-empty">No workers were found.</div>}</div>; }

function formatHarnessValue(value: unknown): string { if (typeof value === "string" || typeof value === "number") return String(value); if (value && typeof value === "object") { const [key, inner] = Object.entries(value)[0] ?? []; return inner === undefined ? String(key ?? "unknown") : `${key}: ${String(inner)}`; } return "unknown"; }
function HarnessList({ discovery, loading, error, query }: { discovery?: HarnessDiscoveryPayload; loading: boolean; error: string; query: string }) { if (loading) return <div className="run-status">Inspecting known harness homes through Rust `enforcer-install` detection.</div>; if (error) return <div className="index-error">{error}</div>; if (!discovery) return <div className="proof-empty">Adapter discovery has not returned an observation.</div>; const present = discovery.harnesses.filter((harness) => harness.present).length; const normalizedQuery = query.trim().toLowerCase(); const harnesses = normalizedQuery ? discovery.harnesses.filter((harness) => [harness.id, harness.homePath, ...harness.evidence.flatMap((evidence) => [evidence.source, evidence.observation]), ...Object.entries(harness.capabilities ?? {}).flatMap(([name, capability]) => [name, formatHarnessValue(capability.value)])].filter(Boolean).join(" ").toLowerCase().includes(normalizedQuery)) : discovery.harnesses; return <><div className="harness-notice">{discovery.verification}</div><div className="settings-root-row"><span>Discovery runtime</span><code>{discovery.runtime}</code></div><div className="harness-summary"><strong>{present} present</strong><span>{harnesses.length} of {discovery.harnesses.length} known harnesses shown</span></div><div className="harness-list">{harnesses.map((harness) => <details className={harness.present ? "harness-row present" : "harness-row absent"} key={harness.id}><summary><span><Cable size={16} /><strong>{harness.id}</strong><small>{harness.homePath ?? "No conventional home path"}</small></span><em>{harness.present ? "Present" : "Absent"}</em></summary><div className="harness-detail"><div className="harness-evidence">{harness.evidence.map((evidence) => <div key={`${evidence.source}:${evidence.observation}`}><code>{evidence.source}</code><span>{evidence.observation}</span></div>)}</div>{harness.capabilities && <div className="harness-capabilities">{Object.entries(harness.capabilities).map(([name, capability]) => <div key={name}><small>{name.replace(/([A-Z])/g, " $1")}</small><strong>{formatHarnessValue(capability.value)}</strong></div>)}</div>}</div></details>)}{harnesses.length === 0 && <div className="proof-empty">No adapter evidence matches this search.</div>}</div></>; }
function HarnessDetail({ discovery }: { discovery?: HarnessDiscoveryPayload }) { const present = discovery?.harnesses.filter((harness) => harness.present).length ?? 0; return <><div className="detail-heading"><Cable size={20} /><span><strong>Global adapters</strong><small>{discovery ? `${present} present / ${discovery.harnesses.length} inspected` : "No adapter observation"}</small></span></div><p>Harnesses are user-level integrations that may serve many projects and Hub lanes. This view never treats a present home folder as an installed or verified Enforcer adapter.</p><div className="policy-callout"><Cable size={17} /><span>Discovery is live Rust evidence. Adapter installation, repair, registration verification, and lifecycle management are not implemented in the desktop.</span></div><dl className="meta-grid"><dt>Scope</dt><dd>Hub-wide, not project-specific</dd><dt>Available</dt><dd>Presence, capability evidence, and source paths</dd><dt>Missing</dt><dd>Install, repair, verify, and remove commands</dd></dl></>; }

function InboxDetail({ message, lanes, onSendMessage, onAcknowledgeMessage }: { message?: HubPayload["mail"][number]; lanes: HubPayload["lanes"]; onSendMessage: (recipientLane: string, body: string) => Promise<void>; onAcknowledgeMessage: (messageId: string) => Promise<void> }) {
  const [recipientLane, setRecipientLane] = useState("");
  const [body, setBody] = useState("");
  const [dispatching, setDispatching] = useState(false);
  const [acknowledging, setAcknowledging] = useState(false);
  const [actionError, setActionError] = useState("");
  useEffect(() => {
    if (!recipientLane && lanes[0]) setRecipientLane(lanes[0].laneId);
  }, [lanes, recipientLane]);
  async function dispatch() {
    setDispatching(true);
    setActionError("");
    try {
      await onSendMessage(recipientLane, body);
      setBody("");
    } catch (error) {
      setActionError(String(error));
    } finally {
      setDispatching(false);
    }
  }
  async function acknowledge() {
    if (!message) return;
    setAcknowledging(true);
    setActionError("");
    try {
      await onAcknowledgeMessage(message.messageId);
    } catch (error) {
      setActionError(String(error));
    } finally {
      setAcknowledging(false);
    }
  }
  return <><div className="detail-heading"><Inbox size={20} /><span><strong>{message?.messageId ?? "Inbox"}</strong><small>{message ? `${message.fromWriter} to ${message.to ?? "broadcast"}` : "Choose a message or dispatch to a known lane."}</small></span></div>{message ? <><p>{message.body ?? "Message body was not recorded."}</p><dl className="meta-grid"><dt>Sent</dt><dd>{message.ts}</dd><dt>Acknowledged</dt><dd>{message.ackedBy.join(", ") || "not acknowledged"}</dd></dl><button className="secondary-action" onClick={acknowledge} disabled={acknowledging}><CheckCheck size={16} /> {acknowledging ? "Acknowledging..." : "Acknowledge"}</button></> : <div className="proof-empty">No message is selected.</div>}<label className="hub-composer"><span>New coordination message</span><select aria-label="Recipient lane" value={recipientLane} onChange={(event) => setRecipientLane(event.target.value)} disabled={dispatching || lanes.length === 0}>{lanes.map((lane) => <option key={lane.laneId} value={lane.laneId}>{lane.laneId}</option>)}</select><textarea value={body} onChange={(event) => setBody(event.target.value)} placeholder="Write a coordination message" disabled={dispatching || lanes.length === 0} /><button className="primary-action" onClick={dispatch} disabled={dispatching || !recipientLane || !body.trim()}><Send size={16} /> {dispatching ? "Dispatching..." : "Send message"}</button></label>{lanes.length === 0 && <div className="index-error">No live recipient lane is available for a message.</div>}{actionError && <div className="index-error">{actionError}</div>}</>;
}

function HubDetail({ view, hub, laneStates }: { view: Exclude<HubView, "inbox" | "harnesses">; hub?: HubPayload; laneStates: Map<string, string> }) { const copy = view === "lanes" ? ["Lane health", `${hub?.lanes.length ?? 0} observed lanes`, "Status and heartbeat summaries come directly from the typed ledger fold."] : view === "claims" ? ["Claim map", `${hub?.claims.length ?? 0} active claim records`, "Claims are exact ownership records. No synthetic paths are shown."] : view === "tasks" ? ["Task stream", `${hub?.tasks.length ?? 0} latest task records`, "Each row is the newest typed task.update event for one task id. No task is fabricated from a claim or message."] : ["Workers", `${hub?.workers.length ?? 0} observed workers`, "Worker health includes typed runtime state, active task, and latest summary."]; return <><div className="detail-heading"><ShieldCheck size={20} /><span><strong>{copy[0]}</strong><small>{copy[1]}</small></span></div><p>{copy[2]}</p><div className="policy-callout"><ShieldCheck size={17} /><span>Hub stays separate from Project context and may span any number of repositories and harnesses.</span></div>{view === "lanes" && Array.from(laneStates).slice(0, 4).map(([lane, state]) => <div className="settings-root-row" key={lane}><span>{lane}</span><code>{state}</code></div>)}</>; }
