import { ExternalLink, Eye, Focus, Maximize2, Network, RefreshCw, Search, ZoomIn, ZoomOut } from "lucide-react";
import { useEffect, useMemo, useState, type PointerEvent, type ReactElement, type WheelEvent } from "react";
import { graphNodeKinds, type GraphNode, type GraphNodeKind, type GraphSourceSnippet, type ProjectGraph } from "../data/graphAdapter";

const kindLabels: Record<GraphNodeKind, string> = {
  file: "Files",
  function: "Functions",
  method: "Methods",
  class: "Classes",
  struct: "Structs",
  interface: "Interfaces",
  enum: "Enums",
  "type-alias": "Type aliases",
  test: "Tests",
  type: "Types",
  module: "Modules",
  lambda: "Lambdas",
  variable: "Variables",
  constant: "Constants",
};

export function GraphWorkspace({ graph, loading, onOpenRetrieval, onLoadSourceSnippet, onOpenIndex, onRefresh, onFocusProjection, onClearFocus }: { graph: ProjectGraph; loading: boolean; onOpenRetrieval: () => void; onLoadSourceSnippet: (node: GraphNode) => Promise<GraphSourceSnippet>; onOpenIndex: () => void; onRefresh: () => void; onFocusProjection: (query: string) => void; onClearFocus: () => void }): ReactElement {
  const [enabledKinds, setEnabledKinds] = useState<Set<GraphNodeKind>>(
    () => new Set(graphNodeKinds),
  );
  const [selectedNodeId, setSelectedNodeId] = useState("");
  const [showLabels, setShowLabels] = useState(false);
  const [query, setQuery] = useState("");
  const [focusNeighborhood, setFocusNeighborhood] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [dragOrigin, setDragOrigin] = useState<{ x: number; y: number; panX: number; panY: number }>();
  const [sourceSnippet, setSourceSnippet] = useState<GraphSourceSnippet>();
  const [sourceSnippetLoading, setSourceSnippetLoading] = useState(false);
  const [sourceSnippetError, setSourceSnippetError] = useState("");

  const visibleNodes = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return graph.nodes.filter((node) => enabledKinds.has(node.kind) && (!normalizedQuery || `${node.label} ${node.path}`.toLowerCase().includes(normalizedQuery)));
  }, [enabledKinds, graph.nodes, query]);
  const nodeById = useMemo(() => new Map(graph.nodes.map((node) => [node.id, node])), [graph.nodes]);
  const edgeLabels = useMemo(() => [...new Set(graph.edges.map((edge) => edge.label))].sort(), [graph.edges]);
  const [enabledEdgeLabels, setEnabledEdgeLabels] = useState<Set<string>>(
    () => new Set(graph.edges.map((edge) => edge.label)),
  );
  useEffect(() => {
    setEnabledEdgeLabels(new Set(graph.edges.map((edge) => edge.label)));
  }, [graph.edges]);
  const kindCounts = useMemo(() => {
    const counts = new Map<GraphNodeKind, number>();
    for (const node of graph.nodes) counts.set(node.kind, (counts.get(node.kind) ?? 0) + 1);
    return counts;
  }, [graph.nodes]);
  const selectedNode = visibleNodes.find((node) => node.id === selectedNodeId) ?? visibleNodes[0];
  const relatedIds = useMemo(() => new Set(selectedNode ? graph.edges.filter((edge) => edge.from === selectedNode.id || edge.to === selectedNode.id).flatMap((edge) => [edge.from, edge.to]) : []), [graph.edges, selectedNode]);
  const renderedNodes = useMemo(() => focusNeighborhood && selectedNode ? visibleNodes.filter((node) => relatedIds.has(node.id)) : visibleNodes, [focusNeighborhood, relatedIds, selectedNode, visibleNodes]);
  const visibleIds = useMemo(() => new Set(renderedNodes.map((node) => node.id)), [renderedNodes]);
  const visibleEdges = useMemo(() => graph.edges.filter((edge) => enabledEdgeLabels.has(edge.label) && visibleIds.has(edge.from) && visibleIds.has(edge.to)), [enabledEdgeLabels, graph.edges, visibleIds]);
  const projectionSummary = graph.projectionLimited
    ? `${graph.nodes.length.toLocaleString()} rendered of ${graph.totalNodes.toLocaleString()} indexed nodes`
    : `${graph.nodes.length.toLocaleString()} rendered nodes`;
  const nodeRadius = Math.max(0.45, Math.min(3.2, 5 / Math.sqrt(Math.max(renderedNodes.length, 1) / 8)));
  const edgeOpacity = Math.max(0.08, Math.min(0.65, 160 / Math.max(visibleEdges.length, 1)));
  const labelsVisible = showLabels && renderedNodes.length <= 160;
  const viewWidth = 100 / zoom;
  const viewHeight = 80 / zoom;
  const viewX = Math.max(0, Math.min(100 - viewWidth, 50 - viewWidth / 2 + pan.x));
  const viewY = Math.max(0, Math.min(80 - viewHeight, 40 - viewHeight / 2 + pan.y));

  useEffect(() => {
    if (graph.focusNodeId && graph.nodes.some((node) => node.id === graph.focusNodeId)) setSelectedNodeId(graph.focusNodeId);
  }, [graph.focusNodeId, graph.nodes]);

  useEffect(() => {
    let cancelled = false;
    if (!selectedNode) {
      setSourceSnippet(undefined);
      setSourceSnippetLoading(false);
      setSourceSnippetError("");
      return () => { cancelled = true; };
    }
    setSourceSnippet(undefined);
    setSourceSnippetError("");
    setSourceSnippetLoading(true);
    void onLoadSourceSnippet(selectedNode)
      .then((snippet) => { if (!cancelled) setSourceSnippet(snippet); })
      .catch((error: unknown) => { if (!cancelled) setSourceSnippetError(String(error)); })
      .finally(() => { if (!cancelled) setSourceSnippetLoading(false); });
    return () => { cancelled = true; };
  }, [selectedNode?.id, selectedNode?.line, selectedNode?.path]);

  function toggleKind(kind: GraphNodeKind) {
    setEnabledKinds((current) => {
      const next = new Set(current);
      if (next.has(kind) && next.size > 1) next.delete(kind);
      else next.add(kind);
      return next;
    });
  }

  function toggleEdgeLabel(label: string) {
    setEnabledEdgeLabels((current) => {
      const next = new Set(current);
      if (next.has(label) && next.size > 1) next.delete(label);
      else next.add(label);
      return next;
    });
  }

  function changeZoom(nextZoom: number) {
    setZoom(Math.max(1, Math.min(6, nextZoom)));
  }

  function resetView() {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }

  function onGraphWheel(event: WheelEvent<SVGSVGElement>) {
    event.preventDefault();
    changeZoom(zoom + (event.deltaY < 0 ? 0.25 : -0.25));
  }

  function onGraphPointerDown(event: PointerEvent<SVGSVGElement>) {
    if (event.target !== event.currentTarget) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    setDragOrigin({ x: event.clientX, y: event.clientY, panX: pan.x, panY: pan.y });
  }

  function onGraphPointerMove(event: PointerEvent<SVGSVGElement>) {
    if (!dragOrigin) return;
    const bounds = event.currentTarget.getBoundingClientRect();
    setPan({
      x: dragOrigin.panX - ((event.clientX - dragOrigin.x) / bounds.width) * viewWidth,
      y: dragOrigin.panY - ((event.clientY - dragOrigin.y) / bounds.height) * viewHeight,
    });
  }

  if (loading) {
    return <section className="main-surface graph-empty"><Network size={30} /><strong>Reading code index</strong><small>The Memory Explorer is loading the existing X06 projection for {graph.project.name}.</small></section>;
  }

  if (!graph.available) {
    return (
      <section className="main-surface graph-empty">
        <Network size={30} />
        <strong>No code index for this project</strong>
        <small>The graph only shows persisted code-memory data. Create the project index explicitly; this does not scan for Enforcer violations or change rule/doctrine settings.</small>
        <button className="primary-action" onClick={onOpenIndex}><ExternalLink size={16} /> Open index settings</button>
        {graph.error && <details className="graph-error-details"><summary>Technical details</summary><code>{graph.error}</code></details>}
      </section>
    );
  }

  return (
    <section className="main-surface graph-explorer">
      <aside className="graph-filter-panel">
        <div className="panel-head">
          <span><strong>Graph facets</strong><small>Filter the rendered projection, not the project index.</small></span>
        </div>
        <div className="graph-filter-stack">
          {graphNodeKinds.map((kind) => {
            const count = kindCounts.get(kind) ?? 0;
            const enabled = enabledKinds.has(kind);
            return (
              <button
                aria-pressed={enabled}
                className={enabled ? "graph-filter enabled" : "graph-filter"}
                key={kind}
                onClick={() => toggleKind(kind)}
              >
                <span>{kindLabels[kind]}</span><em>{count}</em>
              </button>
            );
          })}
        </div>
        {edgeLabels.length > 0 && <div className="graph-edge-filters">
          <strong>Edge types</strong>
          {edgeLabels.map((label) => <button aria-pressed={enabledEdgeLabels.has(label)} className={enabledEdgeLabels.has(label) ? "graph-edge-filter enabled" : "graph-edge-filter"} key={label} onClick={() => toggleEdgeLabel(label)}>{label}<em>{graph.edges.filter((edge) => edge.label === label).length}</em></button>)}
        </div>}
        {graph.projectionLimited && graph.folderAggregates.length > 0 && <div className="graph-folder-map">
          <strong>Folder map</strong><small>Rust-computed aggregates for the full index. Open a folder to load its focused projection.</small>
          <div className="graph-folder-list">{graph.folderAggregates.map((folder) => <button key={folder.path} onClick={() => onFocusProjection(folder.path)} title={`Open ${folder.path}`}><span>{folder.path}</span><em>{folder.files} files / {folder.symbols} symbols / {folder.calls} calls</em></button>)}</div>
        </div>}
        <div className="graph-mini-stats">
          <span><strong>{renderedNodes.length.toLocaleString()}</strong><small>rendered nodes</small></span>
          <span><strong>{visibleEdges.length.toLocaleString()}</strong><small>projection links</small></span>
          <span><strong>{graph.filesIndexed.toLocaleString()}</strong><small>indexed files</small></span>
        </div>
      </aside>
      <div className="graph-canvas graph-canvas-large">
        <div className="panel-head">
          <span><strong>{graph.project.name} code graph</strong><small>{projectionSummary} / {graph.totalEdges.toLocaleString()} stored call, import, or route edges.</small></span>
          <div className="action-row tight">
            <label className="graph-search"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Find code" aria-label="Find graph node" /></label>
            <button className="icon-button" title="Toggle labels for small graph views" aria-label="Toggle labels" aria-pressed={showLabels} onClick={() => setShowLabels((value) => !value)}><Eye size={15} /></button>
            <button className="ghost-button" aria-pressed={focusNeighborhood} disabled={!selectedNode} onClick={() => setFocusNeighborhood((value) => !value)}><Focus size={15} /> Focus</button>
            {graph.projectionLimited && !graph.focusQuery && <button className="ghost-button" disabled={!query.trim()} onClick={() => onFocusProjection(query.trim())}><Search size={15} /> Load focus</button>}
            {graph.focusQuery && <button className="ghost-button" onClick={onClearFocus}><Maximize2 size={15} /> Full projection</button>}
            <button className="icon-button" title="Zoom out" aria-label="Zoom out" disabled={zoom <= 1} onClick={() => changeZoom(zoom - 0.5)}><ZoomOut size={15} /></button>
            <button className="icon-button" title="Zoom in" aria-label="Zoom in" disabled={zoom >= 6} onClick={() => changeZoom(zoom + 0.5)}><ZoomIn size={15} /></button>
            <button className="icon-button" title="Reset graph view" aria-label="Reset graph view" onClick={resetView}><Maximize2 size={15} /></button>
            <button className="icon-button" title="Reread the persisted native graph projection; this does not re-index the project" aria-label="Refresh graph" onClick={onRefresh}><RefreshCw size={15} /></button>
          </div>
        </div>
        {(graph.projectionLimited || graph.focusQuery || (showLabels && !labelsVisible)) && <div className="graph-limit-notice">{graph.focusQuery ? graph.focusMatched ? `Focused project projection for "${graph.focusQuery}". This canvas contains matching indexed files and their symbols.` : `No indexed path, symbol, or call matched "${graph.focusQuery}". The canvas is intentionally empty.` : graph.projectionLimited ? `Showing ${graph.nodes.length.toLocaleString()} of ${graph.totalNodes.toLocaleString()} indexed nodes. Use Folder map or Load focus to inspect a bounded project area without pretending the full graph fits on one canvas.` : "Labels are suppressed above 160 rendered nodes. Narrow the graph or focus a node to reveal them."}</div>}
        <div className="node-map deep graph-node-map">
          <svg viewBox={`${viewX} ${viewY} ${viewWidth} ${viewHeight}`} role="img" aria-label={`${graph.project.name} knowledge graph`} onWheel={onGraphWheel} onPointerDown={onGraphPointerDown} onPointerMove={onGraphPointerMove} onPointerUp={() => setDragOrigin(undefined)} onPointerCancel={() => setDragOrigin(undefined)}>
            {visibleEdges.map((edge) => {
              const from = nodeById.get(edge.from);
              const to = nodeById.get(edge.to);
              if (!from || !to) return null;
              return <line key={`${edge.from}-${edge.to}`} x1={from.x} y1={from.y} x2={to.x} y2={to.y} className="graph-edge" style={{ opacity: edgeOpacity }} />;
            })}
            {renderedNodes.map((node) => {
              const selected = node.id === selectedNode?.id;
              const related = relatedIds.has(node.id);
              return (
                <g
                  className={`graph-node ${node.kind} ${selected ? "selected" : ""} ${related ? "related" : ""}`}
                  key={node.id}
                  onClick={() => setSelectedNodeId(node.id)}
                  onKeyDown={(event) => event.key === "Enter" && setSelectedNodeId(node.id)}
                  role="button"
                  tabIndex={0}
                >
                  <circle cx={node.x} cy={node.y} r={selected ? nodeRadius * 1.8 : nodeRadius} />
                  {(selected || labelsVisible) && <text x={node.x + nodeRadius * 1.4} y={node.y + nodeRadius * 0.35}>{selected ? node.label : node.label.slice(0, 24)}</text>}
                </g>
              );
            })}
          </svg>
        </div>
      </div>
      {selectedNode && (
        <aside className="graph-detail-panel">
          <div className="detail-heading"><Network size={20} /><strong>Selected node</strong></div>
          <div className="graph-node-title"><span className={`node-kind-dot ${selectedNode.kind}`} /> <h2>{selectedNode.label}</h2></div>
          <p className="detail-copy">{selectedNode.summary}</p>
          <dl className="meta-grid">
            <dt>Kind</dt><dd>{kindLabels[selectedNode.kind]}</dd>
            <dt>Status</dt><dd>{selectedNode.status}</dd>
            <dt>Path</dt><dd>{selectedNode.path}:{selectedNode.line}</dd>
            <dt>Links</dt><dd>{Math.max(relatedIds.size - 1, 0)}</dd>
          </dl>
          <div className="graph-source-preview">
            <strong>Source excerpt</strong>
            {sourceSnippetLoading && <small>Reading project-relative source.</small>}
            {sourceSnippetError && <small className="source-preview-error">{sourceSnippetError}</small>}
            {sourceSnippet && <><small>{sourceSnippet.path}:{sourceSnippet.startLine}-{sourceSnippet.endLine}</small><div className="code-preview"><code>{sourceSnippet.content}</code></div></>}
          </div>
          <div className="graph-related-list">
            <strong>Connected objects</strong>
            {[...relatedIds].filter((id) => id !== selectedNode.id).map((id) => {
              const node = nodeById.get(id);
              return node ? <button key={id} onClick={() => setSelectedNodeId(id)}>{node.label}<span>{kindLabels[node.kind]}</span></button> : null;
            })}
          </div>
          <button className="primary-action graph-chat-link" onClick={onOpenRetrieval}><ExternalLink size={16} /> Search and retrieval</button>
        </aside>
      )}
    </section>
  );
}
