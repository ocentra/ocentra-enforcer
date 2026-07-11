import { ArrowUpRight, BrainCircuit, Cpu, MessageSquare, Network, Search, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { GraphWorkspace } from "./GraphWorkspace";
import type { GraphFocus, GraphNode, GraphSourceSnippet, ProjectGraph } from "../data/graphAdapter";

type MemoryTab = "graph" | "retrieval" | "conversation" | "learning" | "models" | "parity";
type MemoryTabDefinition = { key: MemoryTab; label: string; icon: typeof Network };

export type MemorySummaryPayload = {
  provenance: { scope: "engine-proof"; selectedProjectRoot: string; artifactRoot: string; generatedAtUnixSecs: number | null };
  retrieval: { available: boolean; status: string; rowsTotal: number; rowsGreen: number; rowsDegraded: number };
  learning: { available: boolean; status: string; lessons: number; blockers: number; followUps: number };
  models: { available: boolean; runtimeMode: string; allowNetwork: boolean; cacheRoot: string; observations: number };
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

type GraphSearchPayload = { total: number; hasMore: boolean; results: Array<{ nodeId: string; name: string; qualifiedName: string; label: string; filePath: string; rank?: number }> };

export function MemoryExplorerWorkspace({ graph, graphLoading, summary, summaryLoading, search, searchLoading, searchError, onSearch, onLoadSourceSnippet, onOpenIndex, onRefreshGraph, onFocusGraph, onClearGraphFocus }: { graph: ProjectGraph; graphLoading: boolean; summary?: MemorySummaryPayload; summaryLoading: boolean; search?: GraphSearchPayload; searchLoading: boolean; searchError: string; onSearch: (query: string) => Promise<void>; onLoadSourceSnippet: (node: GraphNode) => Promise<GraphSourceSnippet>; onOpenIndex: () => void; onRefreshGraph: () => void; onFocusGraph: (focus: GraphFocus) => void; onClearGraphFocus: () => void }) {
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

function MemoryEvidencePanel({ tab, summary, loading, search, searchLoading, searchError, onSearch, onOpenGraphResult }: { tab: Exclude<MemoryTab, "graph" | "conversation">; summary?: MemorySummaryPayload; loading: boolean; search?: GraphSearchPayload; searchLoading: boolean; searchError: string; onSearch: (query: string) => Promise<void>; onOpenGraphResult: (hit: GraphSearchPayload["results"][number]) => void }) {
  if (loading) return <section className="main-surface memory-empty"><span>Reading Enforcer x06 evidence artifacts.</span></section>;
  if (!summary) return <section className="main-surface memory-empty"><strong>Memory evidence unavailable</strong><span>Open the Enforcer desktop shell to read its checked-in x06 proof artifacts.</span></section>;
  if (tab === "retrieval") return <RetrievalPanel summary={summary} search={search} searchLoading={searchLoading} searchError={searchError} onSearch={onSearch} onOpenGraphResult={onOpenGraphResult} />;
  if (tab === "learning") return <section className="main-surface memory-evidence learning-evidence"><header className="panel-head"><span><strong>Learning evidence</strong><EngineProofSource summary={summary} artifact="x06-learning-curve.json" /></span></header><MetricRow values={[{ label: "Lessons", value: summary.learning.lessons }, { label: "Blockers", value: summary.learning.blockers }, { label: "Follow-ups", value: summary.learning.followUps }]} />{summary.learning.available ? <p className="memory-copy">Projection status: {summary.learning.status || "recorded"}. Detailed lesson entries and their t0, t1, and t2 timelines are not loaded by Rust yet.</p> : <div className="learning-unavailable"><BrainCircuit size={28} /><strong>No learning-curve artifact is available</strong><small>The Enforcer engine has no persisted t0 observation, t1 landed artifact, or t2 recurrence evidence to display.</small></div>}</section>;
  if (tab === "models") return <section className="main-surface memory-evidence"><header className="panel-head"><span><strong>Model capability</strong><EngineProofSource summary={summary} artifact="x06-models.json" /></span></header><MetricRow values={[{ label: "Runtime", value: summary.models.runtimeMode || "unavailable" }, { label: "Network", value: summary.models.allowNetwork ? "allowed" : "disabled" }, { label: "Observations", value: summary.models.observations }]} /><p className="memory-copy">{summary.models.available ? `Cache root: ${summary.models.cacheRoot || "not recorded"}. Model installation, selection, execution, and observation capture are not implemented in the Rust desktop.` : "No Enforcer model capability artifact is available."}</p></section>;
  return <section className="main-surface memory-evidence"><header className="panel-head"><span><strong>KG parity</strong><EngineProofSource summary={summary} artifact="x06-kg-parity.json" /></span></header><MetricRow values={[{ label: "Tools", value: summary.parity.toolsTotal }, { label: "Equal", value: summary.parity.equal }, { label: "Better", value: summary.parity.better }, { label: "Worse", value: summary.parity.worse }, { label: "Incomparable", value: summary.parity.incomparable }]} /><div className="parity-rows">{summary.parity.rows.map((row) => <article key={row.tool}><span className={`parity-verdict ${row.verdict}`}>{row.verdict}</span><strong>{row.tool}</strong>{row.reason && <small>{row.reason}</small>}</article>)}</div></section>;
}

function RetrievalPanel({ summary, search, searchLoading, searchError, onSearch, onOpenGraphResult }: { summary: MemorySummaryPayload; search?: GraphSearchPayload; searchLoading: boolean; searchError: string; onSearch: (query: string) => Promise<void>; onOpenGraphResult: (hit: GraphSearchPayload["results"][number]) => void }) {
  const [query, setQuery] = useState("");
  return <section className="main-surface memory-evidence"><header className="panel-head"><span><strong>Search graph</strong><small>Live deterministic BM25 over the selected project's persisted code graph. It retrieves source evidence; it does not synthesize an LLM answer.</small></span></header><MetricRow values={[{ label: "QA rows", value: summary.retrieval.rowsTotal }, { label: "Green", value: summary.retrieval.rowsGreen }, { label: "Degraded", value: summary.retrieval.rowsDegraded }]} /><form className="settings-add-row" onSubmit={(event) => { event.preventDefault(); if (query.trim()) void onSearch(query); }}><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search with code terms, for example widget configuration" /><button className="primary-action" disabled={searchLoading || !query.trim()}><Search size={16} /> {searchLoading ? "Retrieving" : "Retrieve evidence"}</button></form>{searchError && <div className="index-error">{searchError}</div>}{search && <><div className="policy-callout"><Search size={17} /><span>{search.total} graph source{search.total === 1 ? "" : "s"} matched this search. Answer synthesis is not available: open a result to inspect the focused graph and source excerpt.</span></div><div className="retrieval-results" role="list">{search.results.map((hit) => <button className="retrieval-result-row" key={hit.nodeId} onClick={() => onOpenGraphResult(hit)} role="listitem" title={`Open ${hit.name} in the focused code graph`}><span><strong>{hit.name}</strong><small>{hit.label} / {hit.filePath}{hit.rank !== undefined ? ` / rank ${hit.rank.toFixed(2)}` : ""}</small></span><ArrowUpRight size={16} /></button>)}{search.results.length === 0 && <p className="memory-copy">No indexed code matched this query.</p>}</div></>}<p className="memory-copy">{summary.retrieval.available ? <>Engine QA artifact status: {summary.retrieval.status || "recorded"}. Semantic fusion, reranking, context-pack explanation, and model-backed answer synthesis remain separate capability work. <EngineProofSource summary={summary} artifact="x06-rag-qa.json" /></> : "No Enforcer engine retrieval QA artifact is available."}</p></section>;
}

function EngineProofSource({ summary, artifact }: { summary: MemorySummaryPayload; artifact: string }) {
  const generatedAt = summary.provenance.generatedAtUnixSecs === null ? "time unavailable" : new Date(summary.provenance.generatedAtUnixSecs * 1000).toLocaleString();
  return <small title={`Scope: ${summary.provenance.scope}. Selected project: ${summary.provenance.selectedProjectRoot}. Artifact root: ${summary.provenance.artifactRoot}.`}>Engine proof, not the selected project / {artifact} / generated {generatedAt}.</small>;
}

function MetricRow({ values }: { values: Array<{ label: string; value: string | number }> }) {
  return <div className="memory-metrics">{values.map((item) => <span key={item.label}><strong>{typeof item.value === "number" ? item.value.toLocaleString() : item.value}</strong><small>{item.label}</small></span>)}</div>;
}
