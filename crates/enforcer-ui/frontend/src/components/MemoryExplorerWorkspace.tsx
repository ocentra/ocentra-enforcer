import { ArrowUpRight, BrainCircuit, Cpu, MessageSquare, Network, Search, ShieldCheck } from "lucide-react";
import { type ReactElement, useState } from "react";
import { GraphWorkspace } from "./GraphWorkspace";
import type { GraphFocus, GraphNode, GraphSourceSnippet, ProjectGraph } from "../data/graphAdapter";

type MemoryTab = "graph" | "retrieval" | "conversation" | "learning" | "models" | "parity";
type MemoryTabDefinition = { key: MemoryTab; label: string; icon: typeof Network };

export type MemorySummaryPayload = {
  provenance: { scope: string; selectedProjectRoot: string; artifactRoot: string; generatedAtUnixSecs: number | null };
  projectGraph: { available: boolean; projectScope: string; storeRoot: string; nodes: number; edges: number; files: number; codeGraphItems: number; memoryEvidenceItems: number; status: string; reason: string };
  retrieval: { available: boolean; status: string; rowsTotal: number; rowsGreen: number; rowsDegraded: number; tokenReductionEstimate: string; explanations: Array<{ id: string; query: string; capabilityState: string; expectedIds: string[]; actualIds: string[]; sourceRefs: string[]; bm25Candidates: number; vectorCandidates: number; rrfScore?: string; rerankerScore?: string; selectedContextPack: string }> };
  learning: { available: boolean; status: string; lessons: Array<{ lessonId: string; lesson: string; status: string; evidence: string[] }>; blockers: string[]; followUps: string[]; recurrenceSignals: string[] };
  models: { available: boolean; runtimeMode: string; capabilityState: string; allowNetwork: boolean; cacheRoot: string; observations: number; artifacts: Array<{ artifact: string; status: string; capability: string; reason: string }> };
  parity: { available: boolean; toolsTotal: number; equal: number; better: number; worse: number; incomparable: number; unrunnable: number; rows: Array<{ tool: string; verdict: string; reason: string }> };
};

const projectTabs: MemoryTabDefinition[] = [
  { key: "graph", label: "Code graph", icon: Network },
  { key: "retrieval", label: "Search graph", icon: Search },
  { key: "conversation", label: "Ask graph", icon: MessageSquare },
];

const engineEvidenceTabs: MemoryTabDefinition[] = [
  { key: "learning", label: "Learning evidence", icon: BrainCircuit },
  { key: "models", label: "Models", icon: Cpu },
  { key: "parity", label: "Parity", icon: ShieldCheck },
];

type GraphSearchResponse = { total: number; hasMore: boolean; query: string; projectScope: string; results: Array<{ nodeId: string; name: string; qualifiedName: string; label: string; filePath: string; evidenceKind: "code-graph" | "learning-memory" | "proof-artifact"; rank?: string }> };

export function MemoryExplorerWorkspace({ graph, graphLoading, summary, summaryLoading, search, searchLoading, searchError, onSearch, onLoadSourceSnippet, onOpenIndex, onRefreshGraph, onFocusGraph, onClearGraphFocus }: { graph: ProjectGraph; graphLoading: boolean; summary?: MemorySummaryPayload | undefined; summaryLoading: boolean; search?: GraphSearchResponse | undefined; searchLoading: boolean; searchError: string; onSearch: (query: string) => Promise<void>; onLoadSourceSnippet: (node: GraphNode) => Promise<GraphSourceSnippet>; onOpenIndex: () => void; onRefreshGraph: () => void; onFocusGraph: (focus: GraphFocus) => void; onClearGraphFocus: () => void }): ReactElement {
  const [tab, setTab] = useState<MemoryTab>("graph");
  return (
    <section className="memory-explorer">
      <header className="memory-header">
        <div><strong>Memory Explorer</strong><small>Project code graph and source retrieval are separate from Enforcer Engine evidence and X06 parity proof.</small></div>
        <div className="memory-tab-groups" role="tablist" aria-label="Memory Explorer tools">
          <div className="memory-tab-group"><small>Project intelligence</small><div className="memory-tabs">{projectTabs.map(({ key, label, icon: Icon }) => <button key={key} role="tab" aria-selected={tab === key} className={tab === key ? "memory-tab active" : "memory-tab"} onClick={() => setTab(key)}><Icon size={15} /> {label}</button>)}</div></div>
          <div className="memory-tab-group"><small>Engine evidence</small><div className="memory-tabs">{engineEvidenceTabs.map(({ key, label, icon: Icon }) => <button key={key} role="tab" aria-selected={tab === key} className={tab === key ? "memory-tab active" : "memory-tab"} onClick={() => setTab(key)}><Icon size={15} /> {label}</button>)}</div></div>
        </div>
      </header>
      {tab === "graph" && <GraphWorkspace graph={graph} loading={graphLoading} onOpenRetrieval={() => setTab("retrieval")} onLoadSourceSnippet={onLoadSourceSnippet} onOpenIndex={onOpenIndex} onRefresh={onRefreshGraph} onFocusProjection={(query) => onFocusGraph({ query })} onClearFocus={onClearGraphFocus} />}
      {tab === "conversation" && <GraphConversationPanel onOpenSearch={() => setTab("retrieval")} />}
      {(tab === "retrieval" || tab === "learning" || tab === "models" || tab === "parity") && <MemoryEvidencePanel tab={tab} summary={summary} loading={summaryLoading} search={search} searchLoading={searchLoading} searchError={searchError} onSearch={onSearch} onOpenGraphResult={(hit) => { onFocusGraph({ query: hit.name, nodeId: hit.nodeId }); setTab("graph"); }} />}
    </section>
  );
}

function GraphConversationPanel({ onOpenSearch }: { onOpenSearch: () => void }) {
  return <section className="main-surface memory-evidence graph-conversation"><header className="panel-head"><span><strong>Ask graph</strong><small>Selected-project graph conversation</small></span></header><div className="conversation-boundary"><MessageSquare size={30} /><strong>Graph conversation is not implemented in Rust</strong><p>The desktop can retrieve deterministic source evidence from the selected project's stored graph. It cannot yet assemble a context pack, invoke an answer runtime, persist a conversation, or claim that a generated answer is grounded.</p><dl><dt>Available now</dt><dd>Search graph, focus matching code, inspect source evidence.</dd><dt>Missing Rust boundary</dt><dd>Context-pack contract, answer session, model execution, citation and observation persistence.</dd><dt>Plan placement</dt><dd>g09 Memory KG/RAG Explorer.</dd></dl><button className="primary-action" onClick={onOpenSearch}><Search size={16} /> Open search graph</button></div></section>;
}

function MemoryEvidencePanel({ tab, summary, loading, search, searchLoading, searchError, onSearch, onOpenGraphResult }: { tab: Exclude<MemoryTab, "graph" | "conversation">; summary?: MemorySummaryPayload | undefined; loading: boolean; search?: GraphSearchResponse | undefined; searchLoading: boolean; searchError: string; onSearch: (query: string) => Promise<void>; onOpenGraphResult: (hit: GraphSearchResponse["results"][number]) => void }) {
  if (loading) return <section className="main-surface memory-empty"><span>Reading Enforcer x06 evidence artifacts.</span></section>;
  if (!summary) return <section className="main-surface memory-empty"><strong>Memory evidence unavailable</strong><span>Open the Enforcer desktop shell to read its checked-in x06 proof artifacts.</span></section>;
  if (tab === "retrieval") return <RetrievalPanel summary={summary} search={search} searchLoading={searchLoading} searchError={searchError} onSearch={onSearch} onOpenGraphResult={onOpenGraphResult} />;
  if (tab === "learning") return <section className="main-surface memory-evidence learning-evidence"><header className="panel-head"><span><strong>Learning evidence</strong><EngineProofSource summary={summary} artifact="x06-learning-curve.json" /></span></header><MetricRow values={[{ label: "Lessons", value: summary.learning.lessons.length }, { label: "Blockers", value: summary.learning.blockers.length }, { label: "Follow-ups", value: summary.learning.followUps.length }, { label: "Signals", value: summary.learning.recurrenceSignals.length }]} />{summary.learning.available ? <><p className="memory-copy">Projection status: {summary.learning.status || "recorded"}. Rust distinguishes these learning records from selected-project code graph nodes.</p><div className="parity-rows">{summary.learning.lessons.map((lesson) => <article key={lesson.lessonId}><span className="parity-verdict better">{lesson.status}</span><strong>{lesson.lessonId}</strong><small>{lesson.lesson}</small><small>{lesson.evidence.join(" / ")}</small></article>)}</div></> : <div className="learning-unavailable"><BrainCircuit size={28} /><strong>No learning-curve artifact is available</strong><small>The Enforcer engine has no persisted t0 observation, t1 landed artifact, or t2 recurrence evidence to display.</small></div>}</section>;
  if (tab === "models") return <section className="main-surface memory-evidence"><header className="panel-head"><span><strong>Model capability</strong><EngineProofSource summary={summary} artifact="x06-models.json" /></span></header><MetricRow values={[{ label: "Runtime", value: summary.models.runtimeMode || "unavailable" }, { label: "State", value: summary.models.capabilityState }, { label: "Network", value: summary.models.allowNetwork ? "allowed" : "disabled" }, { label: "Observations", value: summary.models.observations }]} /><p className="memory-copy">{summary.models.available ? `Cache root: ${summary.models.cacheRoot || "not recorded"}. This panel reads proof artifacts only; passive render does not start downloads or model processes.` : "No Enforcer model capability artifact is available."}</p><div className="parity-rows">{summary.models.artifacts.map((artifact) => <article key={artifact.artifact}><span className={`parity-verdict ${artifact.status === "present" ? "better" : "incomparable"}`}>{artifact.status}</span><strong>{artifact.artifact}</strong><small>{artifact.capability}</small><small>{artifact.reason}</small></article>)}</div></section>;
  return <section className="main-surface memory-evidence"><header className="panel-head"><span><strong>KG parity</strong><EngineProofSource summary={summary} artifact="x06-kg-parity.json" /></span></header><MetricRow values={[{ label: "Tools", value: summary.parity.toolsTotal }, { label: "Equal", value: summary.parity.equal }, { label: "Better", value: summary.parity.better }, { label: "Worse", value: summary.parity.worse }, { label: "Incomparable", value: summary.parity.incomparable }]} /><div className="parity-rows">{summary.parity.rows.map((row) => <article key={row.tool}><span className={`parity-verdict ${row.verdict}`}>{row.verdict}</span><strong>{row.tool}</strong>{row.reason && <small>{row.reason}</small>}</article>)}</div></section>;
}

function RetrievalPanel({ summary, search, searchLoading, searchError, onSearch, onOpenGraphResult }: { summary: MemorySummaryPayload; search?: GraphSearchResponse | undefined; searchLoading: boolean; searchError: string; onSearch: (query: string) => Promise<void>; onOpenGraphResult: (hit: GraphSearchResponse["results"][number]) => void }) {
  const [query, setQuery] = useState("");
  return <section className="main-surface memory-evidence"><header className="panel-head"><span><strong>Search graph</strong><small>Live deterministic search over the selected project's persisted code graph, with x06 RAG proof explained separately.</small></span></header><MetricRow values={[{ label: "QA rows", value: summary.retrieval.rowsTotal }, { label: "Green", value: summary.retrieval.rowsGreen }, { label: "Degraded", value: summary.retrieval.rowsDegraded }, { label: "Context pack", value: summary.retrieval.explanations[0]?.selectedContextPack ?? "none" }]} /><form className="settings-add-row" onSubmit={(event) => { event.preventDefault(); if (query.trim()) void onSearch(query); }}><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search with code terms, for example widget configuration" /><button className="primary-action" disabled={searchLoading || !query.trim()}><Search size={16} /> {searchLoading ? "Retrieving" : "Retrieve evidence"}</button></form>{searchError && <div className="index-error">{searchError}</div>}{search && <><div className="policy-callout"><Search size={17} /><span>{search.total} {search.results[0]?.evidenceKind ?? "code-graph"} source{search.total === 1 ? "" : "s"} matched this selected-project search.</span></div><div className="retrieval-results" role="list">{search.results.map((hit) => <button className="retrieval-result-row" key={hit.nodeId} onClick={() => onOpenGraphResult(hit)} role="listitem" title={`Open ${hit.name} in the focused code graph`}><span><strong>{hit.name}</strong><small>{hit.evidenceKind} / {hit.label} / {hit.filePath}{hit.rank !== undefined ? ` / rank ${hit.rank}` : ""}</small></span><ArrowUpRight size={16} /></button>)}{search.results.length === 0 && <p className="memory-copy">No indexed code matched this query.</p>}</div></>}<div className="parity-rows">{summary.retrieval.explanations.map((row) => <article key={row.id}><span className="parity-verdict better">{row.capabilityState || "proof"}</span><strong>{row.id}: {row.query}</strong><small>BM25 {row.bm25Candidates} / vector {row.vectorCandidates} / RRF {row.rrfScore ?? "n/a"} / rerank {row.rerankerScore ?? "n/a"}</small><small>{row.sourceRefs.join(" / ")}</small></article>)}</div><p className="memory-copy">{summary.retrieval.available ? <>Engine QA artifact status: {summary.retrieval.status || "recorded"}. Token reduction: {summary.retrieval.tokenReductionEstimate}. <EngineProofSource summary={summary} artifact="x06-rag-qa.json" /></> : "No Enforcer engine retrieval QA artifact is available."}</p></section>;
}

function EngineProofSource({ summary, artifact }: { summary: MemorySummaryPayload; artifact: string }) {
  const generatedAt = summary.provenance.generatedAtUnixSecs === null ? "time unavailable" : new Date(summary.provenance.generatedAtUnixSecs * 1000).toLocaleString();
  return <small title={`Scope: ${summary.provenance.scope}. Selected project: ${summary.provenance.selectedProjectRoot}. Artifact root: ${summary.provenance.artifactRoot}.`}>Engine proof, not the selected project / {artifact} / generated {generatedAt}.</small>;
}

function MetricRow({ values }: { values: Array<{ label: string; value: string | number }> }) {
  return <div className="memory-metrics">{values.map((item) => <span key={item.label}><strong>{typeof item.value === "number" ? item.value.toLocaleString() : item.value}</strong><small>{item.label}</small></span>)}</div>;
}
