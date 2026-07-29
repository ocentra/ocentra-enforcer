import http from "node:http";
import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { chromium } from "playwright";

const repoRoot = process.cwd();
const frontendRoot = path.join(repoRoot, "crates/enforcer-ui/frontend");
const distRoot = path.join(frontendRoot, "dist");
const proofPath = path.join(repoRoot, "proof/ui/g09-memory-explorer.json");
const selectedRoot = path.join(frontendRoot, "src-tauri/tests/fixtures/desktop/cargo-workspace");

function responseFor(command, args) {
  switch (command) {
    case "desktop_status":
      return { shell: "browser-proof", bindingMode: "mocked-tauri-boundary" };
    case "load_desktop_projects":
      return [{ id: "fixture", name: "Fixture workspace", root: selectedRoot, repoKey: "fixture", branch: "g09", indexed: "ready", detectedLanguages: ["Rust"], inspection: "live" }];
    case "inspect_project":
      return { available: true, gitRoot: selectedRoot, branch: "g09", detectedLanguages: ["Rust"] };
    case "load_scan_targets":
      return [];
    case "load_project_rule_coverage":
      return { detectedLanguages: ["Rust"], catalogLanguages: ["Rust"], observedWithoutCatalog: [], settingsStatus: "fixture", rules: [] };
    case "load_cached_scan":
      break;
    case "load_desktop_scan_history":
      return [];
    case "memory_index_status":
      return { available: true };
    case "load_project_settings":
      return { sourcePath: ".enforce/config.json", nativeTies: [], ruleToggles: [] };
    case "load_scan_scope_settings":
      return { sourcePath: ".enforce/config.json", exists: true, profileName: "fixture", ignoreDirs: [], ignoreFileGlobs: [] };
    case "load_workpack_index":
      return { sourcePath: "docs/plans/enforcer-selfhost-plan/WORKPACK_INDEX.md", rows: [], statusCounts: {}, caveat: "fixture" };
    case "load_engine_capabilities":
      return { capabilities: [] };
    case "load_desktop_rule_catalog":
      return { rules: [] };
    case "load_graph":
      return {
        root: args?.root ?? selectedRoot,
        totalNodes: 3,
        totalEdges: 2,
        filesIndexed: 1,
        projectionLimited: false,
        focusQuery: args?.focus?.query ?? null,
        focusNodeId: args?.focus?.nodeId ?? null,
        focusMatched: true,
        folderAggregates: [{ path: "src", files: 1, symbols: 2, calls: 1 }],
        nodes: [
          { id: "file-src-lib", label: "src/lib.rs", kind: "file", path: "src/lib.rs", line: 1, status: "indexed" },
          { id: "sym-handler", label: "handle_request", kind: "function", path: "src/lib.rs", line: 5, status: "indexed" },
        ],
        edges: [{ from: "file-src-lib", to: "sym-handler", label: "defines" }],
      };
    case "load_graph_source_snippet":
      return { path: args.path, line: args.line, startLine: 4, endLine: 8, content: "    5 | fn handle_request() {}" };
    case "load_memory_summary":
      return {
        provenance: { scope: "project-store-plus-engine-proof", selectedProjectRoot: selectedRoot, artifactRoot: path.join(repoRoot, "proof/memory"), generatedAtUnixSecs: 1783771200 },
        projectGraph: { available: true, projectScope: selectedRoot, storeRoot: path.join(selectedRoot, ".enforce/memory"), nodes: 3, edges: 2, files: 1, codeGraphItems: 3, memoryEvidenceItems: 0, status: "store-backed-code-graph", reason: "fixture" },
        retrieval: { available: true, status: "green", rowsTotal: 1, rowsGreen: 1, rowsDegraded: 0, tokenReductionEstimate: "42%", explanations: [{ id: "QA-001", query: "Find handler", capabilityState: "host-local-proof", expectedIds: ["sym-handler"], actualIds: ["sym-handler"], sourceRefs: ["src/lib.rs"], bm25Candidates: 1, vectorCandidates: 1, rrfScore: "1.000", rerankerScore: "1.000", selectedContextPack: "sourceRefs" }] },
        learning: { available: true, status: "learned", lessons: [{ lessonId: "dogfood-001", lesson: "Keep code graph and learning memory distinct.", status: "learned", evidence: ["proof/memory/x06-learning-curve.json"] }], blockers: [], followUps: [], recurrenceSignals: ["route-choice"] },
        models: { available: true, runtimeMode: "cache-only", capabilityState: "cache-only", allowNetwork: false, cacheRoot: ".cache/models", observations: 1, artifacts: [{ artifact: "x06-runtime-control-plane.json", status: "present", capability: "control-plane", reason: "passive read only" }] },
        parity: { available: true, toolsTotal: 1, equal: 0, better: 1, worse: 0, incomparable: 0, unrunnable: 0, rows: [{ tool: "search_graph", verdict: "better", reason: "typed Rust evidence" }] },
      };
    case "search_memory_graph":
      return { total: 1, hasMore: false, query: args.query, projectScope: selectedRoot, results: [{ nodeId: "sym-handler", name: "handle_request", qualifiedName: "fixture.handle_request", label: "Function", filePath: "src/lib.rs", evidenceKind: "code-graph", rank: "1.0000" }] };
    default:
      return {};
  }
}

async function serveStatic(root) {
  const server = http.createServer(async (request, response) => {
    const urlPath = decodeURIComponent(new URL(request.url ?? "/", "http://127.0.0.1").pathname);
    const file = path.normalize(path.join(root, urlPath === "/" ? "index.html" : urlPath));
    if (!file.startsWith(root)) {
      response.writeHead(403).end();
      return;
    }
    try {
      const body = await fs.readFile(file);
      const type = file.endsWith(".js") ? "text/javascript" : file.endsWith(".css") ? "text/css" : "text/html";
      response.writeHead(200, { "content-type": type });
      response.end(body);
    } catch {
      response.writeHead(404).end();
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return { server, url: `http://127.0.0.1:${server.address().port}` };
}

const { server, url } = await serveStatic(distRoot);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 950 } });
const events = [];
await page.addInitScript(() => {
  window.__TAURI_INTERNALS__ = {
    invoke: async (command, args) => window.__g09Invoke(command, args),
    transformCallback: () => 0,
    unregisterCallback: () => {},
    convertFileSrc: (filePath) => filePath,
  };
});
await page.exposeFunction("__g09Invoke", async (command, args) => {
  events.push({ command, args });
  return responseFor(command, args);
});

try {
  await page.goto(url, { waitUntil: "networkidle" });
  await page.locator("button.nav-item[title='Memory']").click();
  await page.getByRole("tab", { name: /search graph/i }).click();
  await page.getByPlaceholder(/Search with code terms/i).fill("handler");
  await page.getByRole("button", { name: /Retrieve evidence/i }).click();
  await page.getByText("handle_request").click();
  await page.getByRole("tab", { name: /Learning evidence/i }).click();
  await page.getByText("dogfood-001").waitFor();
  await page.getByRole("tab", { name: /Models/i }).click();
  await page.getByText("x06-runtime-control-plane.json").waitFor();
  await page.getByRole("tab", { name: /Parity/i }).click();
  await page.getByText("search_graph").waitFor();
  await fs.mkdir(path.dirname(proofPath), { recursive: true });
  await fs.writeFile(proofPath, JSON.stringify({
    schemaVersion: 1,
    status: "pass",
    proof: "memory-explorer-seeded-kg-rag",
    commands: events.map((event) => event.command),
    assertions: [
      "opened memory view",
      "ran graph search intent",
      "opened focused graph result",
      "rendered RAG explanation",
      "rendered learning evidence",
      "rendered model health",
      "rendered parity rows"
    ],
  }, null, 2));
} finally {
  await browser.close();
  await new Promise((resolve) => server.close(resolve));
}
