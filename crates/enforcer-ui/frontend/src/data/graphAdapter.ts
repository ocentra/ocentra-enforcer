import type { Project } from "./enforcerAppData";

export type GraphNodeKind = "file" | "function" | "method" | "class" | "struct" | "interface" | "enum" | "type-alias" | "type" | "module" | "test" | "lambda" | "variable" | "constant";

export type GraphNode = {
  id: string;
  label: string;
  kind: GraphNodeKind;
  path: string;
  line: number;
  x: number;
  y: number;
  status: string;
  summary: string;
  snippet: string;
};

export type GraphEdge = { from: string; to: string; label: string };

export type GraphSourceSnippet = {
  path: string;
  line: number;
  startLine: number;
  endLine: number;
  content: string;
};

export type GraphFocus = { query: string; nodeId?: string };
export type GraphFolderAggregate = { path: string; files: number; symbols: number; calls: number };

type NativeGraphNode = Omit<GraphNode, "x" | "y" | "summary" | "snippet">;

export type NativeGraphPayload = {
  root: string;
  totalNodes: number;
  totalEdges: number;
  filesIndexed: number;
  folderAggregates: GraphFolderAggregate[];
  nodes: NativeGraphNode[];
  edges: GraphEdge[];
  projectionLimited: boolean;
  focusQuery?: string;
  focusNodeId?: string;
  focusMatched: boolean;
};

export type ProjectGraph = {
  project: Project;
  root: string;
  totalNodes: number;
  totalEdges: number;
  filesIndexed: number;
  folderAggregates: GraphFolderAggregate[];
  projectionLimited: boolean;
  focusQuery?: string;
  focusNodeId?: string;
  focusMatched: boolean;
  available: boolean;
  error?: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
};

export const graphNodeKinds: GraphNodeKind[] = ["file", "function", "method", "class", "struct", "interface", "enum", "type-alias", "type", "module", "test", "lambda", "variable", "constant"];

function stableHash(value: string): number {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

export function graphFromNative(project: Project, payload: NativeGraphPayload): ProjectGraph {
  const sorted = [...payload.nodes].sort((left, right) => left.id.localeCompare(right.id));
  const files = sorted.filter((node) => node.kind === "file");
  const symbolsByPath = new Map<string, NativeGraphNode[]>();
  for (const node of sorted) {
    if (node.kind === "file") continue;
    const group = symbolsByPath.get(node.path) ?? [];
    group.push(node);
    symbolsByPath.set(node.path, group);
  }
  const fileCenters = new Map<string, { x: number; y: number }>();
  files.forEach((file, index) => {
    const compactIndex = files.length <= 12;
    const angle = compactIndex
      ? ((index / Math.max(files.length, 1)) * Math.PI * 2) - Math.PI / 2
      : index * 2.399963229728653;
    const radius = compactIndex
      ? (files.length === 1 ? 0 : 26)
      : Math.min(35, 7 + Math.sqrt(index + 0.5) * 1.2);
    fileCenters.set(file.path, {
      x: 50 + Math.cos(angle) * radius,
      y: 40 + Math.sin(angle) * radius * 0.74,
    });
  });

  const nodes = sorted.map((node) => {
    const center = fileCenters.get(node.path) ?? { x: 50, y: 40 };
    const siblings = symbolsByPath.get(node.path) ?? [];
    const index = siblings.findIndex((item) => item.id === node.id);
    const angle = ((stableHash(node.id) % 360) / 180) * Math.PI;
    const radius = node.kind === "file"
      ? 0
      : siblings.length <= 24
        ? Math.min(12, 4.8 + Math.sqrt(Math.max(index, 0) + 1) * 1.8)
        : Math.min(7.5, 1.35 + Math.sqrt(Math.max(index, 0) + 1) * 0.85);
    return {
      ...node,
      x: center.x + Math.cos(angle) * radius,
      y: center.y + Math.sin(angle) * radius,
      summary: `${node.kind} indexed from ${node.path}:${node.line}.`,
      snippet: node.path ? `${node.path}:${node.line}` : node.id,
    };
  });

  const visibleIds = new Set(nodes.map((node) => node.id));
  return {
    project,
    root: payload.root,
    totalNodes: payload.totalNodes,
    totalEdges: payload.totalEdges,
    filesIndexed: payload.filesIndexed,
    folderAggregates: payload.folderAggregates,
    projectionLimited: payload.projectionLimited,
    focusQuery: payload.focusQuery,
    focusNodeId: payload.focusNodeId,
    focusMatched: payload.focusMatched,
    available: true,
    nodes,
    edges: payload.edges.filter((edge) => visibleIds.has(edge.from) && visibleIds.has(edge.to)),
  };
}

export function unavailableGraph(project: Project, error?: string): ProjectGraph {
  return {
    project,
    root: project.root,
    totalNodes: 0,
    totalEdges: 0,
    filesIndexed: 0,
    folderAggregates: [],
    projectionLimited: false,
    focusMatched: false,
    available: false,
    error,
    nodes: [],
    edges: [],
  };
}
