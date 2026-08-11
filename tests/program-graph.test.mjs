import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import { buildGraphFromConfig, deriveState, listBlocked, listReady, validateGraph } from "../scripts/program-graph.mjs";

function fixture(rows, extra = {}) {
  const root = mkdtempSync(join(tmpdir(), "enforcer-program-graph-"));
  mkdirSync(join(root, "docs", "plans", "fixture-plan", "workpacks"), { recursive: true });
  for (const row of rows) writeFileSync(join(root, "docs", "plans", "fixture-plan", "workpacks", `${row.toLowerCase()}.md`), `# ${row}\n`);
  writeFileSync(join(root, "docs", "plans", "fixture-plan", "README.md"), "# Fixture plan\n");
  writeFileSync(join(root, "docs", "plans", "fixture-plan", "WORKPACK_INDEX.md"), [
    "# Workpack Index",
    "",
    "| Status | ID | Workpack | Depends on |",
    "|---|---|---|---|",
    ...rows.map((row) => `| ${extra[row]?.status ?? "PLANNED"} | ${row} | [${row}](workpacks/${row.toLowerCase()}.md) | ${extra[row]?.depends ?? "none"} |`)
  ].join("\n"));
  const config = {
    schemaVersion: 1,
    graphId: "GOAL/test",
    goal: { id: "GOAL/test", title: "Test goal", path: "docs/plans/fixture-plan/README.md" },
    planRoot: "docs/plans",
    programs: [],
    crossEdges: [],
    artifacts: [],
    policies: {
      declaredDone: ["DONE", "ACCEPTED"],
      declaredActive: ["ACTIVE"],
      declaredPaused: ["PAUSED"],
      declaredReady: ["READY"]
    }
  };
  return { root, config };
}

test("a done dependency unlocks a planned workpack", () => {
  const { root, config } = fixture(["CP00", "CP01"], {
    CP00: { status: "DONE" },
    CP01: { depends: "CP00" }
  });
  const graph = buildGraphFromConfig(root, config);
  const node = graph.nodes.get("WP/fixture-plan/CP01");
  assert.equal(deriveState(graph, node), "ready");
  assert.equal(listReady(graph).length, 1);
});

test("an unsatisfied dependency is visible as a blocker", () => {
  const { root, config } = fixture(["CP00", "CP01"], { CP01: { depends: "CP00" } });
  const graph = buildGraphFromConfig(root, config);
  const blocked = listBlocked(graph);
  assert.equal(blocked.length, 1);
  assert.ok(blocked.find((node) => node.id.endsWith("/CP01")).reasons.some((reason) => reason.id.endsWith("/CP00")));
});

test("cycles fail graph validation", () => {
  const { root, config } = fixture(["CP00", "CP01"], {
    CP00: { depends: "CP01" },
    CP01: { depends: "CP00" }
  });
  const result = validateGraph(buildGraphFromConfig(root, config));
  assert.equal(result.valid, false);
  assert.ok(result.issues.some((issue) => issue.code === "dependency-cycle"));
});

test("a done node with a missing contract path cannot validate", () => {
  const { root, config } = fixture(["CP00"], { CP00: { status: "DONE" } });
  writeFileSync(join(root, "docs", "plans", "fixture-plan", "WORKPACK_INDEX.md"), [
    "# Workpack Index", "", "| Status | ID | Workpack | Depends on |", "|---|---|---|---|",
    "| DONE | CP00 | [CP00](workpacks/missing.md) | none |"
  ].join("\n"));
  const result = validateGraph(buildGraphFromConfig(root, config));
  assert.equal(result.valid, false);
  assert.ok(result.issues.some((issue) => issue.code === "missing-path" || issue.code === "incomplete-done-contract"));
});

test("missing dependency references are actionable", () => {
  const { root, config } = fixture(["CP00"], { CP00: { depends: "CP99" } });
  const result = validateGraph(buildGraphFromConfig(root, config));
  assert.equal(result.valid, false);
  assert.ok(result.issues.some((issue) => issue.code === "missing-edge-target"));
});

test("ALL tracks expands A-H workpacks without unlocking on later cross-cutting tracks", () => {
  const { root, config } = fixture(["a01", "b01", "c01", "x01", "z01"], {
    a01: { status: "DONE" },
    b01: { status: "DONE" },
    z01: { depends: "ALL tracks (A, B, C, D, E, F, G, H)" }
  });
  const graph = buildGraphFromConfig(root, config);
  const dependencies = graph.edges
    .filter((edge) => edge.from === "WP/fixture-plan/z01" && edge.kind === "depends-on")
    .map((edge) => edge.to)
    .sort();

  assert.deepEqual(dependencies, [
    "WP/fixture-plan/a01",
    "WP/fixture-plan/b01",
    "WP/fixture-plan/c01"
  ]);
  assert.equal(deriveState(graph, graph.nodes.get("WP/fixture-plan/z01")), "blocked");
});

test("ALL tracks excludes explicitly opt-in workpacks from the default terminal frontier", () => {
  const { root, config } = fixture(["a01", "e-pack-crypto-blockchain", "z01"], {
    a01: { status: "DONE" },
    z01: { depends: "ALL tracks (A, B, C, D, E, F, G, H)" }
  });
  writeFileSync(join(root, "docs", "plans", "fixture-plan", "WORKPACK_INDEX.md"), [
    "# Workpack Index",
    "",
    "| Status | ID | Workpack | Depends on |",
    "|---|---|---|---|",
    "| DONE | a01 | [a01](workpacks/a01.md) | none |",
    "| TODO | e-pack-crypto-blockchain | [e-pack-crypto-blockchain Crypto Pack](workpacks/e-pack-crypto-blockchain.md) **(OPTIONAL / opt-in — OFF by default)** | a01 |",
    "| TODO | z01 | [z01](workpacks/z01.md) | ALL tracks (A, B, C, D, E, F, G, H) |"
  ].join("\n"));
  const graph = buildGraphFromConfig(root, config);
  const dependencies = graph.edges
    .filter((edge) => edge.from === "WP/fixture-plan/z01" && edge.kind === "depends-on")
    .map((edge) => edge.to)
    .sort();

  assert.deepEqual(dependencies, ["WP/fixture-plan/a01"]);
  assert.equal(deriveState(graph, graph.nodes.get("WP/fixture-plan/z01")), "ready");
});

test("configured lifecycle overrides and control-plane nodes are explicit", () => {
  const { root, config } = fixture(["CP00", "CP01"], { CP01: { depends: "CP00" } });
  config.lifecycleOverrides = {
    "WP/fixture-plan/CP00": { state: "done", source: "accepted evidence" }
  };
  config.extraNodes = [{
    id: "WP/graph-bootstrap",
    kind: "workpack",
    title: "Graph bootstrap",
    path: "docs/plans/fixture-plan/README.md",
    state: "active",
    parent: "GOAL/test",
    metadata: { completionContract: { requiredPaths: ["docs/plans/fixture-plan/README.md"], requiredNodes: [] } }
  }];
  config.crossEdges = [{ from: "WP/fixture-plan/CP01", to: "WP/graph-bootstrap", kind: "depends-on", reason: "graph first" }];
  const graph = buildGraphFromConfig(root, config);
  assert.equal(graph.nodes.get("WP/fixture-plan/CP00").lifecycle, "done");
  assert.equal(deriveState(graph, graph.nodes.get("WP/fixture-plan/CP01")), "blocked");
  assert.ok(listBlocked(graph).find((node) => node.id.endsWith("/CP01")).reasons.some((reason) => reason.id === "WP/graph-bootstrap"));
  assert.equal(deriveState(graph, graph.nodes.get("WP/graph-bootstrap")), "active");
  assert.equal(validateGraph(graph).valid, true);
});
