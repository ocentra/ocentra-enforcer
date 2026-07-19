import { ArrowUpRight, CircleAlert, CircleCheck, Clock3, Cpu, FlaskConical, ListFilter, Search } from "lucide-react";
import { type ReactElement, useMemo, useState } from "react";
import type { WorkspaceKey } from "./AppShell";
import type { UiMaybe, UiTextList } from "../data/enforcerAppData";

export type EngineCapability = {
  id: string;
  domain: string;
  title: string;
  state: "live" | "partial" | "planned" | "evidence";
  source: string;
  controls: string;
  missing: string;
  target: UiMaybe<EngineCapabilityTarget>;
  workpacks: UiTextList;
};

export type EngineCapabilityTarget = {
  mode: "project" | "hub";
  workspace: WorkspaceKey;
  subview: UiMaybe<string>;
  projectContext: "required" | "none";
};

type CapabilityFilter = "all" | EngineCapability["state"];
type EngineView = "capabilities" | "workpacks";
const ENGINE_VIEWS: EngineView[] = ["capabilities", "workpacks"];
type WorkpackPlacementFilter = "all" | "mapped" | "unmapped";
type WorkpackIndexRow = { id: string; title: string; status: string; track: string; owns: string; tier: string; dependencies: string; parallelSafeWith: string; sourcePath: string };
type WorkpackIndexPayload = { sourcePath: string; rows: WorkpackIndexRow[]; caveat: string };
type EngineWorkspaceProps = { capabilities: UiMaybe<EngineCapability[]>; loading: boolean; error: string; workpackIndex: UiMaybe<WorkpackIndexPayload>; workpackIndexLoading: boolean; workpackIndexError: string; onNavigate: (target: EngineCapabilityTarget) => void };

const filters: Array<{ id: CapabilityFilter; label: string }> = [
  { id: "all", label: "All" },
  { id: "live", label: "Usable" },
  { id: "partial", label: "Partial" },
  { id: "planned", label: "Planned" },
  { id: "evidence", label: "Evidence" },
];

const stateCopy: Record<EngineCapability["state"], { label: string; icon: typeof CircleCheck }> = {
  live: { label: "usable now", icon: CircleCheck },
  partial: { label: "partially wired", icon: CircleAlert },
  planned: { label: "not implemented", icon: Clock3 },
  evidence: { label: "evidence only", icon: FlaskConical },
};

function formatTarget(target: EngineCapabilityTarget): string {
  const mode = target.mode === "hub" ? "Hub" : "Project";
  const subviewLabels: { [subview: string]: string } = { harnesses: "Adapters", lanes: "Lane Hub" };
  const workspaceLabels: { [key in WorkspaceKey]: string } = {
    overview: "Overview",
    setup: "Setup",
    findings: "Scan",
    projects: "Projects",
    engine: "Engine",
    analysis: "Analysis",
    runs: "Runs",
    rules: "Rules",
    doctrine: "Policy",
    settings: "Settings",
    assurance: "Assurance",
    hub: "Lane Hub",
    proofs: "Proofs",
    memory: "Memory",
  };
  const destination = target.subview ? subviewLabels[target.subview] ?? target.subview : workspaceLabels[target.workspace] ?? target.workspace;
  return `${mode} -> ${destination}`;
}

export function EngineWorkspace({ capabilities, loading, error, workpackIndex, workpackIndexLoading, workpackIndexError, onNavigate }: EngineWorkspaceProps): ReactElement {
  const [engineView, setEngineView] = useState<EngineView>("capabilities");
  const [filter, setFilter] = useState<CapabilityFilter>("all");
  const [selectedId, setSelectedId] = useState("");
  const [selectedWorkpackId, setSelectedWorkpackId] = useState("");
  const [workpackTrack, setWorkpackTrack] = useState("all");
  const [workpackStatus, setWorkpackStatus] = useState("all");
  const [workpackPlacement, setWorkpackPlacement] = useState<WorkpackPlacementFilter>("all");
  const [workpackQuery, setWorkpackQuery] = useState("");
  const [focusedWorkpackIds, setFocusedWorkpackIds] = useState<UiTextList>();

  const filtered = useMemo(() => (capabilities ?? []).filter((item) => filter === "all" || item.state === filter), [capabilities, filter]);
  const selected = filtered.find((item) => item.id === selectedId) ?? filtered[0];
  const counts = useMemo(() => ({
    usable: (capabilities ?? []).filter((item) => item.state === "live").length,
    partial: (capabilities ?? []).filter((item) => item.state === "partial").length,
    planned: (capabilities ?? []).filter((item) => item.state === "planned").length,
  }), [capabilities]);
  const tracks = useMemo(() => [...new Set((workpackIndex?.rows ?? []).map((row) => row.track))].sort(), [workpackIndex]);
  const statuses = useMemo(() => [...new Set((workpackIndex?.rows ?? []).map((row) => row.status))].sort(), [workpackIndex]);
  const mappedWorkpackIds = useMemo(() => new Set((capabilities ?? []).flatMap((capability) => capability.workpacks)), [capabilities]);
  const workpackPlacementCounts = useMemo(() => ({
    mapped: (workpackIndex?.rows ?? []).filter((row) => mappedWorkpackIds.has(row.id)).length,
    unmapped: (workpackIndex?.rows ?? []).filter((row) => !mappedWorkpackIds.has(row.id)).length,
  }), [mappedWorkpackIds, workpackIndex]);
  const visibleWorkpacks = useMemo(() => {
    const normalizedQuery = workpackQuery.trim().toLowerCase();
    return (workpackIndex?.rows ?? []).filter((row) => (!focusedWorkpackIds || focusedWorkpackIds.includes(row.id)) && (workpackTrack === "all" || row.track === workpackTrack) && (workpackStatus === "all" || row.status === workpackStatus) && (workpackPlacement === "all" || (workpackPlacement === "mapped") === mappedWorkpackIds.has(row.id)) && (!normalizedQuery || [row.id, row.title, row.track, row.status, row.owns, row.dependencies, row.parallelSafeWith].join(" ").toLowerCase().includes(normalizedQuery)));
  }, [focusedWorkpackIds, mappedWorkpackIds, workpackIndex, workpackPlacement, workpackQuery, workpackStatus, workpackTrack]);
  const selectedWorkpack = visibleWorkpacks.find((row) => row.id === selectedWorkpackId) ?? visibleWorkpacks[0];
  const linkedCapabilities = useMemo(() => selectedWorkpack ? (capabilities ?? []).filter((capability) => capability.workpacks.includes(selectedWorkpack.id)) : [], [capabilities, selectedWorkpack]);
  function openCapabilitySurface(capability: EngineCapability) {
    const target = capability.target;
    if (!target) return;
    if (target.workspace === "engine") {
      setWorkpackTrack("all");
      setWorkpackStatus("all");
      setWorkpackPlacement("all");
      setWorkpackQuery("");
      setFocusedWorkpackIds(capability.workpacks);
      setSelectedWorkpackId(capability.workpacks[0] ?? "");
      setEngineView("workpacks");
      return;
    }
    onNavigate(target);
  }

  return <section className="main-surface engine-workspace">
    <div className="engine-heading"><span><strong>Engine capability map</strong><small>What exists in Rust, what this desktop can control, and the exact missing engine boundary.</small></span><Cpu size={20} /></div>
    <div className="engine-metrics" aria-label="Capability summary"><span><strong>{capabilities?.length ?? 0}</strong><small>mapped domains</small></span><span><strong>{counts.usable}</strong><small>usable now</small></span><span><strong>{counts.partial}</strong><small>partially wired</small></span><span><strong>{counts.planned}</strong><small>not implemented</small></span></div>
    <div className="engine-toolbar"><ListFilter size={16} /><div className="segmented-control" aria-label="Engine view">{ENGINE_VIEWS.map((view) => <button key={view} className={engineView === view ? "active" : ""} onClick={() => { if (view === "workpacks") setFocusedWorkpackIds(undefined); setEngineView(view); }}>{view === "capabilities" ? "Capabilities" : "Workpacks"}</button>)}</div><small>{engineView === "capabilities" ? "Rust-provided product metadata" : "Rust read model of the authored plan index"}</small></div>
    <div className="engine-content">
      {engineView === "capabilities" && <CapabilityView capabilities={capabilities} error={error} filter={filter} filtered={filtered} loading={loading} selected={selected} onFilter={setFilter} onOpenSurface={openCapabilitySurface} onOpenWorkpack={(id) => { setFocusedWorkpackIds(undefined); setWorkpackQuery(id); setSelectedWorkpackId(id); setEngineView("workpacks"); }} onSelect={setSelectedId} />}
      {engineView === "workpacks" && <WorkpackView capabilities={capabilities} index={workpackIndex} loading={workpackIndexLoading} error={workpackIndexError} statuses={statuses} tracks={tracks} status={workpackStatus} track={workpackTrack} placement={workpackPlacement} placementCounts={workpackPlacementCounts} query={workpackQuery} focusedWorkpackIds={focusedWorkpackIds} mappedWorkpackIds={mappedWorkpackIds} selected={selectedWorkpack} visible={visibleWorkpacks} linkedCapabilities={linkedCapabilities} onClearFocus={() => setFocusedWorkpackIds(undefined)} onQuery={setWorkpackQuery} onStatus={setWorkpackStatus} onTrack={setWorkpackTrack} onPlacement={setWorkpackPlacement} onSelect={setSelectedWorkpackId} onOpenSurface={openCapabilitySurface} />}
    </div>
  </section>;
}

function CapabilityView({ capabilities, error, filter, filtered, loading, selected, onFilter, onOpenSurface, onOpenWorkpack, onSelect }: { capabilities: UiMaybe<EngineCapability[]>; error: string; filter: CapabilityFilter; filtered: EngineCapability[]; loading: boolean; selected: UiMaybe<EngineCapability>; onFilter: (filter: CapabilityFilter) => void; onOpenSurface: (capability: EngineCapability) => void; onOpenWorkpack: (id: string) => void; onSelect: (id: string) => void }): ReactElement {
  const target = selected?.target;
  const targetLabel = target ? target.workspace === "engine" ? "Open planning workpacks" : `Open ${formatTarget(target)}` : "";
  return <>
    <div className="engine-toolbar compact-toolbar"><ListFilter size={16} /><div className="segmented-control" aria-label="Capability state filter">{filters.map((item) => <button key={item.id} className={filter === item.id ? "active" : ""} onClick={() => onFilter(item.id)}>{item.label}</button>)}</div></div>
    {loading && <div className="run-status">Loading engine capability map.</div>}
    {error && <div className="index-error">{error}</div>}
    {!loading && !error && !capabilities?.length && <div className="proof-empty">The capability catalog is unavailable. No capability state is inferred from the frontend.</div>}
    {!!selected && <div className="engine-layout">
      <div className="engine-catalog" role="list" aria-label="Engine capability catalog">{filtered.map((item) => {
        const copy = stateCopy[item.state];
        const Icon = copy.icon;
        return <button className={selected.id === item.id ? "engine-row selected" : "engine-row"} key={item.id} onClick={() => onSelect(item.id)} role="listitem"><span className={`engine-state ${item.state}`}><Icon size={14} /> {copy.label}</span><strong>{item.title}</strong><small>{item.domain}</small></button>;
      })}</div>
      <aside className="detail-panel engine-detail"><div className="detail-heading"><Cpu size={20} /><span><strong>{selected.title}</strong><small>{selected.domain}</small></span></div><div className={`engine-detail-state ${selected.state}`}>{stateCopy[selected.state].label}</div><dl className="engine-facts"><dt>Rust source</dt><dd>{selected.source}</dd><dt>Desktop controls</dt><dd>{selected.controls}</dd><dt>Target</dt><dd>{target ? formatTarget(target) : "No desktop target"}</dd><dt>Context</dt><dd>{target?.projectContext === "required" ? "selected project required" : "global control-plane context"}</dd><dt>Missing boundary</dt><dd>{selected.missing}</dd><dt>Workpacks</dt><dd>{selected.workpacks.map((workpack) => <button className="workpack-link" key={workpack} onClick={() => onOpenWorkpack(workpack)} title={`Open declared workpack ${workpack}`}><code>{workpack}</code><ArrowUpRight size={12} /></button>)}</dd></dl>{target ? <button className="icon-command" aria-label={targetLabel} title={targetLabel} onClick={() => onOpenSurface(selected)}><ArrowUpRight size={17} /><span>{targetLabel}</span></button> : <div className="policy-callout"><CircleAlert size={16} /><span>No desktop surface exists yet because the required Rust capability is not implemented.</span></div>}</aside>
    </div>}
  </>;
}

function WorkpackView({ capabilities, error, focusedWorkpackIds, index, linkedCapabilities, loading, mappedWorkpackIds, placement, placementCounts, query, selected, status, statuses, track, tracks, visible, onClearFocus, onOpenSurface, onPlacement, onQuery, onSelect, onStatus, onTrack }: { capabilities: UiMaybe<EngineCapability[]>; error: string; focusedWorkpackIds: UiMaybe<UiTextList>; index: UiMaybe<WorkpackIndexPayload>; linkedCapabilities: EngineCapability[]; loading: boolean; mappedWorkpackIds: Set<string>; placement: WorkpackPlacementFilter; placementCounts: { mapped: number; unmapped: number }; query: string; statuses: UiTextList; status: string; tracks: UiTextList; track: string; visible: WorkpackIndexRow[]; selected: UiMaybe<WorkpackIndexRow>; onClearFocus: () => void; onOpenSurface: (capability: EngineCapability) => void; onPlacement: (placement: WorkpackPlacementFilter) => void; onQuery: (query: string) => void; onSelect: (id: string) => void; onStatus: (status: string) => void; onTrack: (track: string) => void }): ReactElement {
  if (loading) return <div className="run-status">Reading declared workpack index.</div>;
  if (error) return <div className="index-error">{error}</div>;
  if (!index) return <div className="proof-empty">The declared workpack index is unavailable.</div>;
  return <>
    <div className="workpack-caveat">{index.caveat}</div>
    {focusedWorkpackIds && <div className="policy-callout"><ListFilter size={16} /><span>Showing {visible.length} workpacks mapped by the selected engine capability.</span><button className="secondary-action" onClick={onClearFocus}>Show all workpacks</button></div>}
    <div className="workpack-controls" aria-label="Workpack filters">
      <label className="workpack-search"><Search size={16} /><input value={query} onChange={(event) => onQuery(event.target.value)} placeholder="Search workpacks, crates, dependencies" aria-label="Search declared workpacks" /></label>
      <label className="workpack-filter"><span>Track</span><select value={track} onChange={(event) => onTrack(event.target.value)} aria-label="Workpack track"><option value="all">All tracks</option>{tracks.map((item) => <option key={item} value={item}>{item}</option>)}</select></label>
      <label className="workpack-filter"><span>Plan state</span><select value={status} onChange={(event) => onStatus(event.target.value)} aria-label="Workpack declared status"><option value="all">All states</option>{statuses.map((item) => <option key={item} value={item}>{item}</option>)}</select></label>
      <label className="workpack-filter"><span>Capability mapping</span><select value={placement} onChange={(event) => onPlacement(workpackPlacementFromValue(event.target.value))} aria-label="Workpack capability mapping"><option value="all">All mappings</option><option value="mapped">Mapped</option><option value="unmapped">Not mapped</option></select></label>
      <small>{visible.length} declared / {placementCounts.mapped} mapped / {placementCounts.unmapped} not mapped</small>
    </div>
    <div className="engine-layout workpack-layout"><div className="engine-catalog" role="list" aria-label="Declared workpack index">{visible.map((row) => <button className={selected?.id === row.id ? "engine-row selected" : "engine-row"} key={row.id} onClick={() => onSelect(row.id)} role="listitem"><span className={`workpack-status ${row.status.toLowerCase()}`}>{row.status}</span><strong>{row.id} {row.title}</strong><small>{row.track} / {row.tier} / {mappedWorkpackIds.has(row.id) ? "capability mapped" : "not capability mapped"}</small></button>)}{visible.length === 0 && <div className="proof-empty">No declared workpack matches these filters.</div>}</div>{selected && <WorkpackDetail index={index} linkedCapabilities={linkedCapabilities} selected={selected} onOpenSurface={onOpenSurface} />}</div>
  </>;
}

function WorkpackDetail({ index, linkedCapabilities, selected, onOpenSurface }: { index: WorkpackIndexPayload; linkedCapabilities: EngineCapability[]; selected: WorkpackIndexRow; onOpenSurface: (capability: EngineCapability) => void }): ReactElement {
  return <aside className="detail-panel engine-detail"><div className="detail-heading"><ListFilter size={20} /><span><strong>{selected.id}</strong><small>{selected.track} / {selected.tier} / declared {selected.status}</small></span></div><section className="workpack-surface-links"><div className="detail-heading"><Cpu size={17} /><span><strong>Capability mapping</strong><small>Rust-owned target metadata, not plan status or completion.</small></span></div>{linkedCapabilities.length ? <div className="workpack-capability-list">{linkedCapabilities.map((capability) => capability.target ? <button key={capability.id} onClick={() => onOpenSurface(capability)}><span><strong>{capability.title}</strong><small>{capability.state} / {capability.domain} / {formatTarget(capability.target)}</small></span><ArrowUpRight size={16} /></button> : <div key={capability.id}><span><strong>{capability.title}</strong><small>{capability.state} / {capability.domain}</small></span><em>No surface</em></div>)}</div> : <div className="policy-callout"><CircleAlert size={16} /><span>This workpack has no capability mapping. That says nothing about its code, proof, or completion; it only means no current desktop target is intentionally mapped to it.</span></div>}</section><dl className="engine-facts"><dt>Workpack</dt><dd>{selected.title}</dd><dt>Owns</dt><dd><code>{selected.owns}</code></dd><dt>Dependencies</dt><dd>{selected.dependencies}</dd><dt>Parallel frontier</dt><dd>{selected.parallelSafeWith}</dd><dt>Document</dt><dd><code>{selected.sourcePath}</code></dd><dt>Index source</dt><dd><code>{index.sourcePath}</code></dd></dl><div className="policy-callout"><CircleAlert size={16} /><span>Declared status is routing information. Open the backing proof and current engine capability before considering this workpack complete.</span></div></aside>;
}

function workpackPlacementFromValue(value: string): WorkpackPlacementFilter {
  if (value === "mapped") return "mapped";
  if (value === "unmapped") return "unmapped";
  return "all";
}
