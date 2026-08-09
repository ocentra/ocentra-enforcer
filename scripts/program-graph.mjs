#!/usr/bin/env node

/**
 * Repo-owned program graph control plane.
 *
 * This is intentionally dependency-free. Markdown remains the detailed plan
 * and proof authority; this graph imports the existing workpack indexes and
 * derives readiness, blockers, dependents, and completion-contract checks.
 * It is read-only in v1: no command can manufacture DONE or mutate a plan.
 */

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const CONFIG_RELATIVE_PATH = "docs/program-engineering-graph.json";
const VALID_KINDS = new Set(["goal", "program", "plan", "workpack", "artifact"]);
const VALID_LIFECYCLE = new Set(["planned", "active", "validation", "done", "failed", "paused"]);
const DERIVED = new Set(["ready", "blocked", "active", "validation", "done", "failed", "paused", "planned"]);

function slash(value) {
  return value.replaceAll("\\", "/");
}

function relativePath(root, absoluteOrRelative) {
  const absolute = resolve(root, absoluteOrRelative);
  return slash(relative(resolve(root), absolute));
}

function isSafeRelativePath(value) {
  if (typeof value !== "string" || value.trim() === "" || value.includes(":")) return false;
  const normalized = slash(value);
  return !normalized.startsWith("/") && !normalized.split("/").some((part) => part === ".." || part === "");
}

function splitTableRow(line) {
  let value = line.trim();
  if (value.startsWith("|")) value = value.slice(1);
  if (value.endsWith("|")) value = value.slice(0, -1);
  return value.split("|").map((cell) => cell.trim());
}

function isSeparatorRow(cells) {
  return cells.length > 0 && cells.every((cell) => /^:?-{3,}:?$/.test(cell));
}

function markdownLinkPath(value) {
  const match = value.match(/\]\(([^)]+)\)/);
  return match ? match[1].split("#", 1)[0].trim() : null;
}

function headingTitle(text, fallback) {
  const heading = text.match(/^#\s+(.+)$/m);
  return heading ? heading[1].trim() : fallback;
}

function declaredLifecycle(status, policies) {
  const value = String(status ?? "").trim().toUpperCase();
  if (policies.declaredDone?.includes(value)) return "done";
  if (policies.declaredActive?.includes(value)) return "active";
  if (policies.declaredPaused?.includes(value)) return "paused";
  if (value === "VALIDATION") return "validation";
  if (value === "FAILED") return "failed";
  return "planned";
}

function rowIdFromCell(value) {
  const id = String(value ?? "").trim();
  return id.replaceAll("`", "");
}

function parseWorkpackTable(text, indexPath, policies) {
  const lines = text.split(/\r?\n/);
  const rows = [];
  const indexDirectory = indexPath.slice(0, indexPath.lastIndexOf("/"));
  const deriveId = (value) => {
    const plain = String(value ?? "").replace(/\[|\]/g, "").replace(/\([^)]*\)/, "").trim();
    return plain.split(/\s+/, 1)[0] ?? "";
  };
  for (let index = 0; index < lines.length; index += 1) {
    if (!lines[index].trim().startsWith("|")) continue;
    const header = splitTableRow(lines[index]).map((cell) => cell.toLowerCase());
    if (!header.some((cell) => cell.startsWith("status")) || !header.some((cell) => cell.startsWith("workpack"))) continue;
    const separator = lines[index + 1]?.trim();
    if (!separator?.startsWith("|")) continue;
    const position = (predicate) => header.findIndex(predicate);
    const statusIndex = position((cell) => cell.startsWith("status"));
    const idIndex = position((cell) => cell === "id");
    const workpackIndex = position((cell) => cell.startsWith("workpack"));
    const dependsIndex = position((cell) => cell.startsWith("depends") || cell === "deps");
    for (index += 2; index < lines.length; index += 1) {
      const line = lines[index].trim();
      if (!line.startsWith("|")) {
        index -= 1;
        break;
      }
      const cells = splitTableRow(line);
      if (isSeparatorRow(cells) || cells.length <= Math.max(statusIndex, workpackIndex)) continue;
      const workpackCell = cells[workpackIndex];
      const shortId = rowIdFromCell(idIndex >= 0 ? cells[idIndex] : deriveId(workpackCell));
      if (!shortId || shortId.toLowerCase() === "id") continue;
      const link = markdownLinkPath(workpackCell);
      const workpackPath = link ? slash(join(indexDirectory, link)) : indexPath;
      const status = statusIndex >= 0 ? cells[statusIndex] : "PLANNED";
      rows.push({
        shortId,
        title: workpackCell.replace(/\[|\]/g, "").replace(/\([^)]*\)/, "").trim() || shortId,
        path: workpackPath,
        declaredStatus: status,
        lifecycle: declaredLifecycle(status, policies),
        rawDepends: dependsIndex >= 0 ? cells[dependsIndex] : "none"
      });
    }
  }
  return rows;
}

function discoverPlanDirectories(root, planRoot) {
  const absoluteRoot = resolve(root, planRoot);
  if (!existsSync(absoluteRoot)) return [];
  return readdirSync(absoluteRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort((left, right) => left.localeCompare(right));
}

function loadPlan(root, planKey, config) {
  const planDirectory = relativePath(root, join(config.planRoot, planKey));
  const indexPath = relativePath(root, join(planDirectory, "WORKPACK_INDEX.md"));
  const readmeCandidates = ["README.md", "PLAN_STATE.md", "AGENTS.md"];
  let title = planKey;
  let titlePath = null;
  for (const candidate of readmeCandidates) {
    const path = relativePath(root, join(planDirectory, candidate));
    if (!existsSync(resolve(root, path))) continue;
    title = headingTitle(readFileSync(resolve(root, path), "utf8"), planKey);
    titlePath = path;
    break;
  }
  const indexExists = existsSync(resolve(root, indexPath));
  const indexText = indexExists ? readFileSync(resolve(root, indexPath), "utf8") : "";
  return {
    key: planKey,
    path: planDirectory,
    title,
    titlePath,
    indexPath,
    indexExists,
    rows: parseWorkpackTable(indexText, indexPath, config.policies ?? {})
  };
}

function workpackNodeId(planKey, shortId) {
  return `WP/${planKey}/${shortId}`;
}

function planNodeId(planKey) {
  return `PLAN/${planKey}`;
}

function allKnownShortIds(plans) {
  const map = new Map();
  for (const plan of plans) {
    for (const row of plan.rows) {
      const list = map.get(row.shortId.toLowerCase()) ?? [];
      list.push({ planKey: plan.key, id: row.shortId });
      map.set(row.shortId.toLowerCase(), list);
    }
  }
  return map;
}

function expandRanges(text, knownIds) {
  const result = new Set();
  const rangePattern = /\b([A-Za-z]+)(\d{1,3})\s*-\s*([A-Za-z]+)(\d{1,3})\b/g;
  for (const match of text.matchAll(rangePattern)) {
    const leftPrefix = match[1];
    const rightPrefix = match[3] ?? leftPrefix;
    if (leftPrefix.toLowerCase() !== rightPrefix.toLowerCase()) continue;
    const start = Number(match[2]);
    const end = Number(match[4]);
    if (!Number.isInteger(start) || !Number.isInteger(end) || end < start || end - start > 200) continue;
    for (let value = start; value <= end; value += 1) {
      for (const known of knownIds) {
        const matchKnown = known.match(/^(.*?)(\d{1,3})$/);
        if (!matchKnown) continue;
        if (matchKnown[1].toLowerCase() === leftPrefix.toLowerCase() && Number(matchKnown[2]) === value) {
          result.add(known);
        }
      }
    }
  }
  return result;
}

function dependencyReferences(rawDepends, plan, knownIds) {
  const raw = String(rawDepends ?? "").trim();
  if (!raw || /^none$/i.test(raw)) return [];
  const found = new Set(expandRanges(raw, knownIds));
  const sorted = [...knownIds].sort((left, right) => right.length - left.length);
  for (const known of sorted) {
    const escaped = known.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const pattern = new RegExp(`(?<![A-Za-z0-9_-])${escaped}(?![A-Za-z0-9_-])`, "i");
    if (pattern.test(raw)) found.add(known);
  }
  // Preserve an ID-shaped token even when it is not imported. The resulting
  // UNRESOLVED edge makes a missing dependency an actionable validation error
  // instead of silently dropping it.
  for (const token of raw.match(/\b[A-Za-z][A-Za-z0-9-]*\d{1,3}\b/g) ?? []) {
    if (/^[A-Za-z]+\d{1,3}-[A-Za-z]+\d{1,3}$/i.test(token)) continue;
    if (!knownIds.some((known) => known.toLowerCase() === token.toLowerCase())) found.add(token);
  }
  return [...found].sort((left, right) => left.localeCompare(right));
}

function resolveShortDependency(plan, shortId, knownMap) {
  const candidates = knownMap.get(shortId.toLowerCase()) ?? [];
  const local = candidates.find((candidate) => candidate.planKey === plan.key);
  if (local) return { ...local, ambiguous: false };
  if (candidates.length === 1) return { ...candidates[0], ambiguous: false };
  if (candidates.length > 1) return { id: shortId, ambiguous: true };
  return null;
}

function graphPathExists(root, path) {
  return existsSync(resolve(root, path));
}

function buildGraphFromConfig(root, config) {
  const plans = discoverPlanDirectories(root, config.planRoot ?? "docs/plans")
    .map((planKey) => loadPlan(root, planKey, config));
  const knownMap = allKnownShortIds(plans);
  const knownIds = [...knownMap.keys()].map((key) => knownMap.get(key)[0].id);
  const nodes = new Map();
  const edges = [];

  const addNode = (node) => {
    if (nodes.has(node.id)) throw new Error(`duplicate graph node id: ${node.id}`);
    nodes.set(node.id, node);
  };

  addNode({
    id: config.goal.id,
    kind: "goal",
    title: config.goal.title,
    path: config.goal.path,
    lifecycle: "active",
    metadata: { description: config.goal.description ?? "" }
  });

  const programByPlan = new Map();
  for (const program of config.programs ?? []) {
    addNode({
      id: program.id,
      kind: "program",
      title: program.title,
      lifecycle: program.state ?? "planned",
      path: program.path ?? null,
      metadata: { ...program }
    });
    edges.push({ from: config.goal.id, to: program.id, kind: "contains", reason: "Program is part of the overall Enforcer goal." });
    for (const planKey of program.planKeys ?? []) programByPlan.set(planKey, program.id);
  }

  for (const plan of plans) {
    const planId = planNodeId(plan.key);
    addNode({
      id: planId,
      kind: "plan",
      title: plan.title,
      path: plan.path,
      lifecycle: programByPlan.has(plan.key) ? nodes.get(programByPlan.get(plan.key)).lifecycle : "planned",
      metadata: {
        planKey: plan.key,
        indexPath: plan.indexPath,
        indexExists: plan.indexExists,
        titlePath: plan.titlePath,
        workpackCount: plan.rows.length,
        programId: programByPlan.get(plan.key) ?? null
      }
    });
    const owner = programByPlan.get(plan.key) ?? config.goal.id;
    edges.push({ from: owner, to: planId, kind: "contains", reason: "Plan discovered under the configured plan root." });
    if (plan.indexExists) {
      const artifactId = `ARTIFACT/${plan.key}/workpack-index`;
      addNode({ id: artifactId, kind: "artifact", title: `${plan.key} workpack index`, path: plan.indexPath, lifecycle: "done", metadata: { source: "WORKPACK_INDEX.md" } });
      edges.push({ from: planId, to: artifactId, kind: "references", reason: "The plan index is the routing source for workpack rows." });
    }
    const expectationsPath = relativePath(root, join(plan.path, "TEST_PROOF_EXPECTATIONS.md"));
    if (graphPathExists(root, expectationsPath)) {
      const artifactId = `ARTIFACT/${plan.key}/test-proof-expectations`;
      addNode({ id: artifactId, kind: "artifact", title: `${plan.key} test and proof expectations`, path: expectationsPath, lifecycle: "done", metadata: { source: "TEST_PROOF_EXPECTATIONS.md" } });
      edges.push({ from: planId, to: artifactId, kind: "references", reason: "Proof expectations remain in the human-readable plan." });
    }
    for (const row of plan.rows) {
      const id = workpackNodeId(plan.key, row.shortId);
      addNode({
        id,
        kind: "workpack",
        title: row.title,
        path: row.path,
        lifecycle: config.lifecycleOverrides?.[id]?.state ?? row.lifecycle,
        metadata: {
          planKey: plan.key,
          shortId: row.shortId,
          declaredStatus: row.declaredStatus,
          rawDepends: row.rawDepends,
          override: config.lifecycleOverrides?.[id] ?? null,
          completionContract: {
            requiredPaths: [row.path],
            requiredNodes: []
          }
        }
      });
      edges.push({ from: planId, to: id, kind: "contains", reason: "Workpack imported from the plan's index." });
      const dependencies = dependencyReferences(row.rawDepends, plan, knownIds);
      for (const dependency of dependencies) {
        const resolvedDependency = resolveShortDependency(plan, dependency, knownMap);
        if (resolvedDependency && !resolvedDependency.ambiguous) {
          edges.push({ from: id, to: workpackNodeId(resolvedDependency.planKey, resolvedDependency.id), kind: "depends-on", reason: `Imported dependency from ${plan.indexPath}.` });
        } else {
          edges.push({ from: id, to: `UNRESOLVED/${dependency}`, kind: "depends-on", reason: `Unresolved dependency token from ${plan.indexPath}.` });
        }
      }
    }
  }

  for (const extra of config.extraNodes ?? []) {
    if (nodes.has(extra.id)) throw new Error(`duplicate graph node id: ${extra.id}`);
    addNode({
      id: extra.id,
      kind: extra.kind,
      title: extra.title,
      path: extra.path ?? null,
      lifecycle: extra.state ?? "planned",
      metadata: extra.metadata ?? {}
    });
    const parent = extra.parent ?? config.goal.id;
    edges.push({ from: parent, to: extra.id, kind: "contains", reason: "Configured control-plane workpack." });
  }

  for (const artifact of config.artifacts ?? []) {
    if (nodes.has(artifact.id)) continue;
    addNode({ id: artifact.id, kind: "artifact", title: artifact.title, path: artifact.path, lifecycle: "done", metadata: { ...artifact } });
    edges.push({ from: config.goal.id, to: artifact.id, kind: "references", reason: "Configured boss evidence artifact." });
  }
  for (const edge of config.crossEdges ?? []) edges.push({ ...edge });

  const uniqueEdges = [];
  const edgeKeys = new Set();
  for (const edge of edges) {
    const key = `${edge.from}\u0000${edge.to}\u0000${edge.kind}`;
    if (edgeKeys.has(key)) continue;
    edgeKeys.add(key);
    uniqueEdges.push(edge);
  }
  const graph = { schemaVersion: config.schemaVersion, graphId: config.graphId, root, config, plans, nodes, edges: uniqueEdges };
  return graph;
}

function dependencyEdges(graph, nodeId) {
  return graph.edges.filter((edge) => edge.kind === "depends-on" && edge.from === nodeId);
}

function dependentEdges(graph, nodeId) {
  return graph.edges.filter((edge) => edge.kind === "depends-on" && edge.to === nodeId);
}

function nodeById(graph, id) {
  if (graph.nodes.has(id)) return graph.nodes.get(id);
  const exact = [...graph.nodes.values()].filter((node) => node.metadata?.shortId?.toLowerCase() === String(id).toLowerCase());
  if (exact.length === 1) return exact[0];
  if (exact.length > 1) throw new Error(`ambiguous graph id '${id}': ${exact.map((node) => node.id).join(", ")}`);
  throw new Error(`graph node not found: ${id}`);
}

function completionStatus(graph, node) {
  const contract = node.metadata?.completionContract;
  const missingPaths = (contract?.requiredPaths ?? []).filter((path) => !graphPathExists(graph.root, path));
  const missingNodes = (contract?.requiredNodes ?? []).filter((id) => {
    try {
      return deriveState(graph, nodeById(graph, id)) !== "done";
    } catch {
      return true;
    }
  });
  return { ok: missingPaths.length === 0 && missingNodes.length === 0, missingPaths, missingNodes };
}

function deriveState(graph, node, memo = new Map(), stack = new Set()) {
  if (memo.has(node.id)) return memo.get(node.id);
  if (stack.has(node.id)) return "blocked";
  stack.add(node.id);
  let state = node.lifecycle ?? "planned";
  if (node.kind === "workpack") {
    if (state === "done") {
      state = completionStatus(graph, node).ok ? "done" : "blocked";
    } else if (state === "planned") {
      const deps = dependencyEdges(graph, node.id);
      const unsatisfied = deps.some((edge) => {
        const target = graph.nodes.get(edge.to);
        return !target || deriveState(graph, target, memo, stack) !== "done";
      });
      state = unsatisfied ? "blocked" : "ready";
    }
  } else if (node.kind === "plan") {
    const children = graph.edges.filter((edge) => edge.from === node.id && edge.kind === "contains").map((edge) => graph.nodes.get(edge.to)).filter(Boolean);
    if (state === "planned" && children.some((child) => deriveState(graph, child, memo, stack) === "active")) state = "active";
    if (state === "planned" && children.length > 0 && children.every((child) => deriveState(graph, child, memo, stack) === "done")) state = "done";
  }
  if (!DERIVED.has(state)) state = "blocked";
  stack.delete(node.id);
  memo.set(node.id, state);
  return state;
}

function reasonsFor(graph, node) {
  const reasons = [];
  for (const edge of dependencyEdges(graph, node.id)) {
    const target = graph.nodes.get(edge.to);
    if (!target) reasons.push({ type: "missing-dependency", id: edge.to, reason: edge.reason });
    else {
      const state = deriveState(graph, target);
      if (state !== "done") reasons.push({ type: "dependency", id: target.id, state, reason: edge.reason });
    }
  }
  if (node.lifecycle === "done") {
    const contract = completionStatus(graph, node);
    for (const path of contract.missingPaths) reasons.push({ type: "completion-path", path });
    for (const id of contract.missingNodes) reasons.push({ type: "completion-node", id });
  }
  return reasons;
}

function validateGraph(graph) {
  const issues = [];
  const ids = new Set();
  for (const node of graph.nodes.values()) {
    if (ids.has(node.id)) issues.push({ severity: "error", code: "duplicate-id", node: node.id });
    ids.add(node.id);
    if (!VALID_KINDS.has(node.kind)) issues.push({ severity: "error", code: "invalid-kind", node: node.id, value: node.kind });
    if (!VALID_LIFECYCLE.has(node.lifecycle)) issues.push({ severity: "error", code: "invalid-lifecycle", node: node.id, value: node.lifecycle });
    if (node.path !== null && node.path !== undefined) {
      if (!isSafeRelativePath(node.path)) issues.push({ severity: "error", code: "unsafe-path", node: node.id, path: node.path });
      else if (!graphPathExists(graph.root, node.path)) issues.push({ severity: "error", code: "missing-path", node: node.id, path: node.path });
    }
    if (node.lifecycle === "done") {
      const completion = completionStatus(graph, node);
      if (!completion.ok) issues.push({ severity: "error", code: "incomplete-done-contract", node: node.id, ...completion });
    }
  }
  for (const edge of graph.edges) {
    if (!graph.nodes.has(edge.from)) issues.push({ severity: "error", code: "missing-edge-origin", edge });
    if (!graph.nodes.has(edge.to)) issues.push({ severity: "error", code: "missing-edge-target", edge });
    if (!edge.kind) issues.push({ severity: "error", code: "missing-edge-kind", edge });
  }
  const visiting = new Set();
  const visited = new Set();
  const visit = (id, path) => {
    if (visiting.has(id)) {
      issues.push({ severity: "error", code: "dependency-cycle", cycle: [...path, id] });
      return;
    }
    if (visited.has(id)) return;
    visiting.add(id);
    for (const edge of dependencyEdges(graph, id)) if (graph.nodes.has(edge.to)) visit(edge.to, [...path, id]);
    visiting.delete(id);
    visited.add(id);
  };
  for (const node of graph.nodes.values()) if (node.kind === "workpack") visit(node.id, []);
  return { valid: !issues.some((issue) => issue.severity === "error"), issues };
}

function serializeNode(graph, node) {
  return {
    id: node.id,
    kind: node.kind,
    title: node.title,
    path: node.path,
    lifecycle: node.lifecycle,
    state: deriveState(graph, node),
    metadata: node.metadata ?? {},
    dependsOn: dependencyEdges(graph, node.id).map((edge) => ({ id: edge.to, state: graph.nodes.has(edge.to) ? deriveState(graph, graph.nodes.get(edge.to)) : "missing", reason: edge.reason })),
    dependents: dependentEdges(graph, node.id).map((edge) => edge.from),
    reasons: reasonsFor(graph, node)
  };
}

function graphStatus(graph) {
  const counts = Object.fromEntries([...DERIVED].map((state) => [state, 0]));
  const byKind = {};
  const records = [];
  for (const node of graph.nodes.values()) {
    const state = deriveState(graph, node);
    counts[state] = (counts[state] ?? 0) + 1;
    byKind[node.kind] ??= {};
    byKind[node.kind][state] = (byKind[node.kind][state] ?? 0) + 1;
    if (["active", "validation", "ready", "blocked"].includes(state)) records.push({ id: node.id, kind: node.kind, title: node.title, state, reasons: reasonsFor(graph, node) });
  }
  const programs = [...graph.nodes.values()].filter((node) => node.kind === "program").map((node) => serializeNode(graph, node));
  const validation = validateGraph(graph);
  return {
    schemaVersion: graph.schemaVersion,
    graphId: graph.graphId,
    root: graph.root,
    plans: graph.plans.map((plan) => ({ key: plan.key, title: plan.title, indexPath: plan.indexPath, indexExists: plan.indexExists, workpackCount: plan.rows.length })),
    counts,
    byKind,
    programs,
    current: records.sort((left, right) => left.id.localeCompare(right.id)),
    validation: { valid: validation.valid, issueCount: validation.issues.length }
  };
}

function listReady(graph, scope = {}) {
  return [...graph.nodes.values()]
    .filter((node) => node.kind === "workpack")
    .filter((node) => !scope.plan || node.metadata?.planKey === scope.plan || node.id === scope.plan)
    .filter((node) => deriveState(graph, node) === "ready")
    .map((node) => serializeNode(graph, node))
    .sort((left, right) => left.id.localeCompare(right.id));
}

function listBlocked(graph, scope = {}) {
  return [...graph.nodes.values()]
    .filter((node) => node.kind === "workpack")
    .filter((node) => !scope.plan || node.metadata?.planKey === scope.plan || node.id === scope.plan)
    .filter((node) => deriveState(graph, node) === "blocked")
    .map((node) => serializeNode(graph, node))
    .sort((left, right) => left.id.localeCompare(right.id));
}

function loadGraph(root) {
  const configPath = resolve(root, CONFIG_RELATIVE_PATH);
  const config = JSON.parse(readFileSync(configPath, "utf8"));
  return buildGraphFromConfig(root, config);
}

function print(value) {
  process.stdout.write(`${JSON.stringify(value, null, 2)}\n`);
}

function parseArgs(argv) {
  let root = process.cwd();
  let plan = null;
  let index = 0;
  while (index < argv.length) {
    if (argv[index] === "--root") {
      root = resolve(argv[index + 1]);
      index += 2;
    } else if (argv[index] === "--plan") {
      plan = argv[index + 1];
      index += 2;
    } else break;
  }
  return { root, plan, command: argv[index] ?? "status", id: argv[index + 1] ?? null };
}

export { buildGraphFromConfig, deriveState, graphStatus, listBlocked, listReady, loadGraph, nodeById, validateGraph };

/** Execute the read-only graph CLI and return a process-style exit code. */
export function main(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  let graph;
  try {
    graph = loadGraph(args.root);
  } catch (error) {
    print({ ok: false, error: String(error?.message ?? error) });
    return 2;
  }
  try {
    switch (args.command) {
      case "status": print(graphStatus(graph)); return 0;
      case "ready": print({ ready: listReady(graph, args) }); return 0;
      case "blocked": print({ blocked: listBlocked(graph, args) }); return 0;
      case "validate": {
        const validation = validateGraph(graph);
        print(validation);
        return validation.valid ? 0 : 1;
      }
      case "inspect": {
        if (!args.id) throw new Error("inspect requires a graph id");
        print(serializeNode(graph, nodeById(graph, args.id)));
        return 0;
      }
      case "deps": {
        if (!args.id) throw new Error("deps requires a graph id");
        const node = nodeById(graph, args.id);
        print({ id: node.id, dependencies: serializeNode(graph, node).dependsOn });
        return 0;
      }
      case "dependents": {
        if (!args.id) throw new Error("dependents requires a graph id");
        const node = nodeById(graph, args.id);
        print({ id: node.id, dependents: serializeNode(graph, node).dependents });
        return 0;
      }
      case "why": {
        if (!args.id) throw new Error("why requires a graph id");
        const node = nodeById(graph, args.id);
        print({ id: node.id, state: deriveState(graph, node), reasons: reasonsFor(graph, node) });
        return 0;
      }
      case "next": print({ ready: listReady(graph, args), blocked: listBlocked(graph, args) }); return 0;
      default: throw new Error(`unknown graph command: ${args.command}`);
    }
  } catch (error) {
    print({ ok: false, error: String(error?.message ?? error) });
    return 2;
  }
}

const entry = process.argv[1] ? resolve(process.argv[1]) : null;
if (entry === fileURLToPath(import.meta.url)) process.exitCode = main();
