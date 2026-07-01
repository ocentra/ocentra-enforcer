import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const SERVER_PATH = path.join(PACK_ROOT, "mcp", "ocentra-enforcer-mcp.mjs");
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");
const TEST_CLI_MAX_BUFFER = 32 * 1024 * 1024;

test("MCP server lists tools, explains rules, and scans a scoped file", async (t) => {
  const launcherRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "rust-rules-mcp-launcher-"),
  );
  const server = spawn(process.execPath, [SERVER_PATH], {
    cwd: launcherRoot,
    stdio: ["pipe", "pipe", "pipe"],
  });
  t.after(() => {
    server.kill();
  });

  const client = createMcpClient(server);
  const initialized = await client.request(1, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "rust-rules-test", version: "0.0.0" },
  });
  assert.equal(initialized.result.serverInfo.name, "ocentra-enforcer");
  client.notify("notifications/initialized", {});

  const tools = await client.request(2, "tools/list", {});
  const toolNames = tools.result.tools.map((tool) => tool.name).sort();
  for (const expectedTool of [
    "ocentra_enforcer_doctor",
    "ocentra_enforcer_explain",
    "ocentra_enforcer_check",
    "ocentra_enforcer_mcp_status",
    "ocentra_enforcer_coordination_claim",
    "ocentra_enforcer_coordination_closeout",
    "ocentra_enforcer_coordination_compact",
    "ocentra_enforcer_coordination_ensure",
    "ocentra_enforcer_coordination_guard",
    "ocentra_enforcer_coordination_health",
    "ocentra_enforcer_coordination_index",
    "ocentra_enforcer_coordination_inbox",
    "ocentra_enforcer_coordination_init",
    "ocentra_enforcer_coordination_mail",
    "ocentra_enforcer_coordination_message",
    "ocentra_enforcer_coordination_notify",
    "ocentra_enforcer_coordination_peer",
    "ocentra_enforcer_coordination_presence",
    "ocentra_enforcer_coordination_release",
    "ocentra_enforcer_coordination_repair",
    "ocentra_enforcer_coordination_report",
    "ocentra_enforcer_coordination_status",
    "ocentra_enforcer_coordination_streams",
    "ocentra_enforcer_coordination_sync",
    "ocentra_enforcer_coordination_tasks",
    "ocentra_enforcer_coordination_workers",
    "ocentra_enforcer_route",
    "ocentra_enforcer_scan",
    "ocentra_enforcer_run",
    "ocentra_enforcer_run_status",
    "ocentra_enforcer_diagnostics",
    "ocentra_enforcer_last_failure",
    "ocentra_enforcer_artifact",
    "ocentra_enforcer_prune_runs",
    "ocentra_enforcer_reset_runs",
    "ocentra_enforcer_proof_artifact",
    "ocentra_enforcer_proof_claim",
    "ocentra_enforcer_proof_diagnostics",
    "ocentra_enforcer_proof_export",
    "ocentra_enforcer_proof_import_legacy",
    "ocentra_enforcer_proof_inventory",
    "ocentra_enforcer_proof_last_failure",
    "ocentra_enforcer_proof_parity",
    "ocentra_enforcer_proof_prune",
    "ocentra_enforcer_proof_reset",
    "ocentra_enforcer_proof_route",
    "ocentra_enforcer_proof_run",
    "ocentra_enforcer_proof_status",
    "rust_rules_doctor",
    "rust_rules_explain",
    "rust_rules_check",
    "rust_rules_route",
    "rust_rules_scan",
  ]) {
    assert.equal(
      toolNames.includes(expectedTool),
      true,
      `missing MCP tool ${expectedTool}`,
    );
  }
  const checkTool = tools.result.tools.find(
    (tool) => tool.name === "ocentra_enforcer_check",
  );
  assert.equal(
    checkTool.inputSchema.properties.check.enum.includes("import-boundaries"),
    true,
  );
  assert.equal(
    checkTool.inputSchema.properties.check.enum.includes("architecture-policy"),
    true,
  );
  assert.equal(checkTool.inputSchema.properties.staged.type, "boolean");
  assert.equal(checkTool.inputSchema.properties.tracked.type, "boolean");
  assert.equal(checkTool.inputSchema.properties.diagnosticLimit.type, "number");
  assert.equal(checkTool.inputSchema.properties.summaryOnly.type, "boolean");
  assert.deepEqual(checkTool.inputSchema.properties.groupBy.enum, [
    "file",
    "slice",
  ]);
  assert.equal(checkTool.inputSchema.properties.includeScope.type, "boolean");
  const invalidCheckArguments = await client.request(18, "tools/call", {
    name: "ocentra_enforcer_check",
    arguments: {
      check: "rule-coverage",
      unexpected: true,
    },
  });
  assert.equal(invalidCheckArguments.result.isError, true);
  assert.match(
    invalidCheckArguments.result.content[0].text,
    /unexpected argument|unexpected/u,
  );
  const claimTool = tools.result.tools.find(
    (tool) => tool.name === "ocentra_enforcer_coordination_claim",
  );
  assert.deepEqual(claimTool.inputSchema.properties.action.enum, ["claim"]);
  assert.deepEqual(claimTool.inputSchema.properties.operation.enum, [
    "inspect",
    "edit",
    "commit",
    "push",
    "rebase",
    "merge",
    "pr_ready",
  ]);
  assert.deepEqual(claimTool.inputSchema.properties.lockKind.enum, [
    "writeLock",
    "globalWriteLock",
    "branchLease",
    "workReservation",
  ]);
  assert.deepEqual(claimTool.inputSchema.properties.onConflict.enum, [
    "fail",
    "intent",
  ]);
  assert.equal(claimTool.inputSchema.properties.branch.type, "string");
  const releaseTool = tools.result.tools.find(
    (tool) => tool.name === "ocentra_enforcer_coordination_release",
  );
  assert.deepEqual(releaseTool.inputSchema.properties.action.enum, ["release"]);
  const closeoutTool = tools.result.tools.find(
    (tool) => tool.name === "ocentra_enforcer_coordination_closeout",
  );
  assert.deepEqual(closeoutTool.inputSchema.properties.action.enum, ["closeout"]);
  assert.equal(closeoutTool.inputSchema.properties.releaseOwned.type, "boolean");
  assert.equal(closeoutTool.inputSchema.properties.repairStale.type, "boolean");

  const mcpStatus = await client.request(19, "tools/call", {
    name: "ocentra_enforcer_mcp_status",
    arguments: {},
  });
  assert.equal(mcpStatus.result.isError, false);
  const mcpStatusReport = JSON.parse(mcpStatus.result.content[0].text);
  assert.equal(mcpStatusReport.ok, true);
  assert.equal(mcpStatusReport.stale, false);
  assert.equal(mcpStatusReport.writeCompatible, true);
  assert.equal(mcpStatusReport.hashCompatibility.ok, true);
  assert.equal(
    mcpStatusReport.startup.digest,
    mcpStatusReport.current.digest,
  );

  const coordinationRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "enforcer-mcp-coordination-"),
  );
  const coordinationTargetRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "enforcer-mcp-coordination-target-"),
  );
  fs.mkdirSync(path.join(coordinationTargetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(coordinationTargetRoot, "src", "lib.rs"), "", "utf8");
  const coordinationInit = await client.request(20, "tools/call", {
    name: "ocentra_enforcer_coordination_init",
    arguments: {
      stateRoot: coordinationRoot,
      hub: "mcp-hub",
      lane: "codex-a",
    },
  });
  assert.equal(coordinationInit.result.isError, false);
  const coordinationClaim = await client.request(21, "tools/call", {
    name: "ocentra_enforcer_coordination_claim",
    arguments: {
      stateRoot: coordinationRoot,
      hub: "mcp-hub",
      root: coordinationTargetRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      reason: "mcp smoke",
      projectId: "mcp-project",
      repoRoot: coordinationTargetRoot,
      worktreeRoot: coordinationTargetRoot,
      codexThreadId: "thread-mcp",
      codexSessionId: "session-mcp",
    },
  });
  assert.equal(coordinationClaim.result.isError, false);
  const coordinationClaimReport = JSON.parse(coordinationClaim.result.content[0].text);
  const coordinationHealth = await client.request(22, "tools/call", {
    name: "ocentra_enforcer_coordination_health",
    arguments: {
      stateRoot: coordinationRoot,
      hub: "mcp-hub",
      root: coordinationTargetRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      projectId: "mcp-project",
      repoRoot: coordinationTargetRoot,
      worktreeRoot: coordinationTargetRoot,
    },
  });
  assert.equal(coordinationHealth.result.isError, false);
  const healthReport = JSON.parse(coordinationHealth.result.content[0].text);
  assert.equal(healthReport.canInspect, true);
  assert.equal(healthReport.canWriteClaimedPaths, true);
  assert.equal(healthReport.presence.rows[0].projectId, "mcp-project");
  const cliGuardAfterMcpClaim = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "guard",
      "--state-root",
      coordinationRoot,
      "--hub",
      "mcp-hub",
      "--lane",
      "codex-a",
      "--root",
      coordinationTargetRoot,
      "--paths",
      "src/lib.rs",
      "--project-id",
      "mcp-project",
      "--repo-root",
      coordinationTargetRoot,
      "--worktree-root",
      coordinationTargetRoot,
      "--json",
    ],
    { cwd: PACK_ROOT, encoding: "utf8", maxBuffer: TEST_CLI_MAX_BUFFER },
  );
  assert.equal(
    cliGuardAfterMcpClaim.status,
    0,
    cliGuardAfterMcpClaim.stdout || cliGuardAfterMcpClaim.stderr,
  );
  assert.equal(JSON.parse(cliGuardAfterMcpClaim.stdout).result.ok, true);

  const coordinationPresence = await client.request(23, "tools/call", {
    name: "ocentra_enforcer_coordination_presence",
    arguments: {
      stateRoot: coordinationRoot,
    },
  });
  assert.equal(coordinationPresence.result.isError, false);
  const presenceReport = JSON.parse(coordinationPresence.result.content[0].text);
  assert.equal(presenceReport.rows[0].codexThreadId, "thread-mcp");

  const coordinationIndex = await client.request(24, "tools/call", {
    name: "ocentra_enforcer_coordination_index",
    arguments: {
      stateRoot: coordinationRoot,
    },
  });
  assert.equal(coordinationIndex.result.isError, false);
  const indexReport = JSON.parse(coordinationIndex.result.content[0].text);
  assert.equal(indexReport.counts.presenceRows, 1);

  const repairDryRun = await client.request(28, "tools/call", {
    name: "ocentra_enforcer_coordination_repair",
    arguments: {
      stateRoot: coordinationRoot,
      action: "legacy-hash",
    },
  });
  assert.equal(repairDryRun.result.isError, false);
  const repairReport = JSON.parse(repairDryRun.result.content[0].text);
  assert.equal(repairReport.dryRun, true);

  const sequenceRepairDryRun = await client.request(30, "tools/call", {
    name: "ocentra_enforcer_coordination_repair",
    arguments: {
      stateRoot: coordinationRoot,
      action: "sequence",
      owner: coordinationClaimReport.event.writer,
      paths: ["src/lib.rs"],
    },
  });
  assert.equal(sequenceRepairDryRun.result.isError, false);
  const sequenceRepairReport = JSON.parse(
    sequenceRepairDryRun.result.content[0].text,
  );
  assert.equal(sequenceRepairReport.dryRun, true);

  const invalidClaimAction = await client.request(25, "tools/call", {
    name: "ocentra_enforcer_coordination_claim",
    arguments: {
      stateRoot: coordinationRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      action: "release",
    },
  });
  assert.equal(invalidClaimAction.result.isError, true);
  const invalidClaimError = JSON.parse(invalidClaimAction.result.content[0].text);
  assert.equal(
    invalidClaimError.error,
    'coordination claim does not support action="release"; use the matching MCP tool instead.',
  );
  const statusAfterInvalidClaim = await client.request(29, "tools/call", {
    name: "ocentra_enforcer_coordination_status",
    arguments: {
      stateRoot: coordinationRoot,
    },
  });
  const activeClaimsAfterInvalidClaim = JSON.parse(
    statusAfterInvalidClaim.result.content[0].text,
  ).state.ownership.activeClaims;
  assert.equal(activeClaimsAfterInvalidClaim.length, 1);
  assert.equal(activeClaimsAfterInvalidClaim[0].eventId, coordinationClaimReport.event.id);

  const coordinationRelease = await client.request(26, "tools/call", {
    name: "ocentra_enforcer_coordination_release",
    arguments: {
      stateRoot: coordinationRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      reason: "mcp release",
    },
  });
  assert.equal(coordinationRelease.result.isError, false);
  const releaseReport = JSON.parse(coordinationRelease.result.content[0].text);
  assert.equal(releaseReport.event.type, "release");

  const coordinationPresenceAfterRelease = await client.request(27, "tools/call", {
    name: "ocentra_enforcer_coordination_presence",
    arguments: {
      stateRoot: coordinationRoot,
    },
  });
  assert.equal(coordinationPresenceAfterRelease.result.isError, false);
  assert.deepEqual(
    JSON.parse(coordinationPresenceAfterRelease.result.content[0].text).views.byClaimedPath,
    {},
  );

  const closeoutClaim = await client.request(31, "tools/call", {
    name: "ocentra_enforcer_coordination_claim",
    arguments: {
      stateRoot: coordinationRoot,
      hub: "mcp-hub",
      root: coordinationTargetRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      reason: "mcp closeout claim",
      codexThreadId: "thread-mcp-closeout",
    },
  });
  assert.equal(closeoutClaim.result.isError, false);
  const closeout = await client.request(32, "tools/call", {
    name: "ocentra_enforcer_coordination_closeout",
    arguments: {
      stateRoot: coordinationRoot,
      hub: "mcp-hub",
      root: coordinationTargetRoot,
      lane: "codex-a",
      codexThreadId: "thread-mcp-closeout",
      reason: "mcp closeout",
    },
  });
  assert.equal(closeout.result.isError, false);
  assert.equal(
    JSON.parse(closeout.result.content[0].text).remainingClaimCount,
    0,
  );

  const explain = await client.request(3, "tools/call", {
    name: "ocentra_enforcer_explain",
    arguments: { ruleId: "RR-7.3" },
  });
  assert.equal(explain.result.isError, false);
  assert.match(explain.result.content[0].text, /RR-7\.3/u);

  const legacyExplain = await client.request(30, "tools/call", {
    name: "rust_rules_explain",
    arguments: { ruleId: "RR-7.3" },
  });
  assert.equal(legacyExplain.result.isError, false);
  assert.match(legacyExplain.result.content[0].text, /RR-7\.3/u);

  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "rust-rules-mcp-"));
  fs.mkdirSync(path.join(tempRoot, "src"));
  fs.writeFileSync(
    path.join(tempRoot, "rust-rules.config.json"),
    JSON.stringify(
      {
        schemaVersion: 2,
        profileName: "mcp-test",
        enforceWorkspaceFiles: false,
        requireCargoDeny: false,
        publicReexportPolicy: "forbid",
        rustRoots: ["src"],
        importBoundaryPolicies: [
          {
            roots: ["src"],
            forbiddenImports: ["@domain/*"],
          },
        ],
      },
      null,
      2,
    ),
  );
  fs.writeFileSync(
    path.join(tempRoot, "src", "lib.rs"),
    "pub use crate::inner::Thing;\n",
  );

  const scan = await client.request(4, "tools/call", {
    name: "rust_rules_scan",
    arguments: {
      root: tempRoot,
      profile: "ocentra-parent",
      scope: "files",
      files: ["src/lib.rs"],
    },
  });
  assert.equal(scan.result.isError, true);
  assert.match(scan.result.content[0].text, /RR-7\.3/u);

  const route = await client.request(5, "tools/call", {
    name: "ocentra_enforcer_route",
    arguments: {
      root: tempRoot,
      profile: "ocentra-parent",
      scope: "files",
      files: ["src/lib.rs"],
    },
  });
  assert.equal(route.result.isError, false);
  const routeReport = JSON.parse(route.result.content[0].text);
  assert.equal(routeReport.profileName, "ocentra-parent");
  assert.deepEqual(routeReport.docs.sort(), [
    "rules/common/documentation.md#covered-rules",
    "rules/common/security.md#covered-rules",
    "rules/common/source.md#covered-rules",
    "rules/rust/async-runtime.md#covered-rules",
    "rules/rust/domain.md#covered-rules",
    "rules/rust/imports-modules.md#covered-rules",
    "rules/rust/source.md#covered-rules",
  ]);
  assert.equal(
    routeReport.docs.some((doc) => doc.includes("toolchain-cargo")),
    false,
  );
  assert.equal(
    routeReport.docs.some((doc) => doc.includes("dependencies")),
    false,
  );

  const cargoRoute = await client.request(6, "tools/call", {
    name: "ocentra_enforcer_route",
    arguments: {
      root: tempRoot,
      profile: "ocentra-parent",
      scope: "files",
      files: ["Cargo.toml"],
    },
  });
  const cargoRouteReport = JSON.parse(cargoRoute.result.content[0].text);
  assert.deepEqual(cargoRouteReport.docs.sort(), [
    "rules/common/security.md#covered-rules",
    "rules/rust/dependencies.md#covered-rules",
    "rules/rust/toolchain-cargo.md#covered-rules",
  ]);

  const tsRoute = await client.request(60, "tools/call", {
    name: "ocentra_enforcer_route",
    arguments: {
      root: tempRoot,
      scope: "files",
      files: ["src/index.ts", "tests/example.test.ts"],
    },
  });
  const tsRouteReport = JSON.parse(tsRoute.result.content[0].text);
  assert.equal(
    tsRouteReport.docs.includes("rules/typescript/source.md#covered-rules"),
    true,
  );
  assert.equal(
    tsRouteReport.docs.includes("rules/typescript/tests.md#covered-rules"),
    true,
  );

  const explicitRoute = await client.request(7, "tools/call", {
    name: "ocentra_enforcer_route",
    arguments: {
      root: tempRoot,
      ruleId: "RR-7.3",
    },
  });
  const explicitRouteReport = JSON.parse(explicitRoute.result.content[0].text);
  assert.deepEqual(
    explicitRouteReport.rules.map((rule) => rule.id),
    ["RR-7.3"],
  );
  assert.deepEqual(explicitRouteReport.docs, [
    "rules/rust/imports-modules.md#covered-rules",
  ]);

  const unknownRoute = await client.request(8, "tools/call", {
    name: "ocentra_enforcer_route",
    arguments: {
      root: tempRoot,
      scope: "files",
      files: ["README.md"],
    },
  });
  const unknownRouteReport = JSON.parse(unknownRoute.result.content[0].text);
  assert.deepEqual(unknownRouteReport.docs, []);
  assert.deepEqual(unknownRouteReport.rules, []);

  const doctor = await client.request(9, "tools/call", {
    name: "ocentra_enforcer_doctor",
    arguments: {
      root: tempRoot,
      profile: "ocentra-parent",
      scope: "files",
      files: ["src/lib.rs"],
    },
  });
  assert.equal(doctor.result.isError, false);
  assert.match(
    doctor.result.content[0].text,
    /"profileName": "ocentra-parent"/u,
  );

  fs.writeFileSync(
    path.join(tempRoot, "src", "schema.ts"),
    ['import { z } from "zo', 'd";\nexport const value = z.string();\n'].join(
      "",
    ),
  );
  const check = await client.request(90, "tools/call", {
    name: "ocentra_enforcer_check",
    arguments: {
      root: tempRoot,
      check: "no-zod-source",
      scope: "files",
      files: ["src/schema.ts"],
    },
  });
  assert.equal(check.result.isError, true);
  const checkReport = JSON.parse(check.result.content[0].text);
  assert.equal(checkReport.check, "no-zod-source");
  assert.deepEqual(
    [...new Set(checkReport.violations.map((violation) => violation.ruleId))],
    ["TS-1.2"],
  );
  assert.equal(
    checkReport.violations.every(
      (violation) =>
        violation.doc === "rules/typescript/source.md#covered-rules",
    ),
    true,
  );

  const compactCheck = await client.request(901, "tools/call", {
    name: "ocentra_enforcer_check",
    arguments: {
      root: tempRoot,
      check: "no-zod-source",
      scope: "files",
      files: ["src/schema.ts"],
      diagnosticLimit: 0,
      groupBy: "slice",
      includeScope: false,
    },
  });
  assert.equal(compactCheck.result.isError, true);
  const compactCheckReport = JSON.parse(compactCheck.result.content[0].text);
  assert.equal(compactCheckReport.counts.findings, 1);
  assert.equal(compactCheckReport.counts.returned, 0);
  assert.equal(compactCheckReport.counts.truncated, true);
  assert.deepEqual(compactCheckReport.ruleIds, ["TS-1.2"]);
  assert.deepEqual(compactCheckReport.docs, [
    "rules/typescript/source.md#covered-rules",
  ]);
  assert.equal("scope" in compactCheckReport, false);
  assert.equal(compactCheckReport.groups[0].key, "src");

  const validationStatus = await client.request(902, "tools/call", {
    name: "ocentra_enforcer_run_status",
    arguments: {
      root: tempRoot,
      tool: "check",
    },
  });
  const validationStatusReport = JSON.parse(
    validationStatus.result.content[0].text,
  );
  assert.equal(validationStatusReport.summaryType, "validation");
  assert.equal(validationStatusReport.summary.kind, "check");
  assert.equal(validationStatusReport.summary.check, "no-zod-source");
  assert.equal(validationStatusReport.summary.ruleIds.includes("TS-1.2"), true);

  fs.writeFileSync(
    path.join(tempRoot, "src", "web.ts"),
    'import { value } from "@domain/core";\nexport const result = value;\n',
  );
  const importBoundaryCheck = await client.request(91, "tools/call", {
    name: "ocentra_enforcer_check",
    arguments: {
      root: tempRoot,
      check: "import-boundaries",
      scope: "files",
      files: ["src/web.ts"],
    },
  });
  assert.equal(importBoundaryCheck.result.isError, true);
  const importBoundaryReport = JSON.parse(
    importBoundaryCheck.result.content[0].text,
  );
  assert.equal(
    importBoundaryReport.violations.some(
      (violation) => violation.ruleId === "TS-4.1",
    ),
    true,
  );

  fs.mkdirSync(path.join(tempRoot, "packages", "app", "src"), {
    recursive: true,
  });
  fs.mkdirSync(path.join(tempRoot, "packages", "app", "tests", "contract"), {
    recursive: true,
  });
  fs.writeFileSync(
    path.join(tempRoot, "packages", "app", "package.json"),
    JSON.stringify({ name: "@mcp/app" }),
  );
  fs.writeFileSync(
    path.join(tempRoot, "packages", "app", "src", "index.ts"),
    "export const value = 1;\n",
  );
  fs.writeFileSync(
    path.join(tempRoot, "packages", "app", "tests", "unit.test.ts"),
    'test("value", () => expect(1).toBe(1));\n',
  );
  fs.writeFileSync(
    path.join(tempRoot, "packages", "app", "tests", "contract", ".gitkeep"),
    "",
  );
  const strictRequiredTests = await client.request(92, "tools/call", {
    name: "ocentra_enforcer_check",
    arguments: {
      root: tempRoot,
      check: "required-tests",
      scope: "files",
      files: ["packages/app/src/index.ts"],
      strictEmptyTestTrees: true,
    },
  });
  assert.equal(strictRequiredTests.result.isError, true);
  const strictRequiredTestsReport = JSON.parse(
    strictRequiredTests.result.content[0].text,
  );
  assert.equal(
    strictRequiredTestsReport.violations.some(
      (violation) => violation.ruleId === "TEST-2.1",
    ),
    true,
  );

  const invalidRoute = await client.request(10, "tools/call", {
    name: "ocentra_enforcer_route",
    arguments: {
      root: tempRoot,
      scope: "files",
      files: "src/lib.rs",
    },
  });
  assert.equal(invalidRoute.result.isError, true);
  assert.match(
    invalidRoute.result.content[0].text,
    /route request schema validation failed/u,
  );

  const harnessRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "ocentra-enforcer-mcp-harness-"),
  );
  const harnessRun = await client.request(11, "tools/call", {
    name: "ocentra_enforcer_run",
    arguments: {
      root: harnessRoot,
      tool: "tsc",
      command: [
        process.execPath,
        "-e",
        'console.log("mcp-stdout-sentinel"); console.error("src/app.ts(2,1): error TS1005: ; expected."); process.exit(1);',
      ],
    },
  });
  assert.equal(harnessRun.result.isError, true);
  const harnessReport = JSON.parse(harnessRun.result.content[0].text);
  assert.equal(harnessReport.summary.status, "failed");

  const lastFailure = await client.request(12, "tools/call", {
    name: "ocentra_enforcer_last_failure",
    arguments: {
      root: harnessRoot,
    },
  });
  const lastFailureReport = JSON.parse(lastFailure.result.content[0].text);
  assert.equal(lastFailureReport.found, true);
  assert.equal(
    lastFailureReport.diagnostics.some(
      (diagnostic) => diagnostic.ruleId === "TS1005",
    ),
    true,
  );

  const runStatusArtifact = await client.request(1201, "tools/call", {
    name: "ocentra_enforcer_run_status",
    arguments: {
      root: harnessRoot,
      artifact: "stdout",
      limitBytes: 200,
    },
  });
  assert.equal(runStatusArtifact.result.isError, false);
  const runStatusArtifactReport = JSON.parse(
    runStatusArtifact.result.content[0].text,
  );
  assert.equal(runStatusArtifactReport.summaryType, "harness");
  assert.equal(runStatusArtifactReport.artifact.artifact, "stdout");
  assert.match(runStatusArtifactReport.artifact.text, /mcp-stdout-sentinel/u);

  const proofRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "ocentra-enforcer-mcp-proof-"),
  );
  fs.mkdirSync(path.join(proofRoot, "scripts", "test"), { recursive: true });
  fs.writeFileSync(
    path.join(proofRoot, "scripts", "test", "tiny-proof.mjs"),
    "console.log('test-results/tiny-proof/proof.json');\n",
  );
  fs.mkdirSync(path.join(proofRoot, "test-results", "tiny-proof"), {
    recursive: true,
  });
  fs.writeFileSync(
    path.join(proofRoot, "test-results", "tiny-proof", "proof.json"),
    JSON.stringify(
      {
        ok: true,
        claimsProved: ["mcp legacy proof artifact is preserved"],
        claimsNotProved: ["legacy script deletion without parity report"],
      },
      null,
      2,
    ),
  );
  const proofRoute = await client.request(13, "tools/call", {
    name: "ocentra_enforcer_proof_route",
    arguments: {
      root: proofRoot,
      scope: "files",
      files: ["scripts/test/tiny-proof.mjs"],
    },
  });
  assert.equal(proofRoute.result.isError, false);
  const proofRouteReport = JSON.parse(proofRoute.result.content[0].text);
  assert.equal(
    proofRouteReport.proofs.some(
      (proof) => proof.id === "PROOF-LEGACY-SCRIPT-INVENTORY",
    ),
    true,
  );

  const proofRun = await client.request(14, "tools/call", {
    name: "ocentra_enforcer_proof_run",
    arguments: {
      root: proofRoot,
      proofId: "PROOF-COMMAND-GENERIC",
      runId: "mcp-proof-pass",
      command: [process.execPath, "-e", "console.log('mcp-proof')"],
    },
  });
  assert.equal(proofRun.result.isError, false);
  const proofRunReport = JSON.parse(proofRun.result.content[0].text);
  assert.equal(proofRunReport.proofRun.status, "passed");

  const proofStatus = await client.request(15, "tools/call", {
    name: "ocentra_enforcer_proof_status",
    arguments: {
      root: proofRoot,
      proofId: "PROOF-COMMAND-GENERIC",
    },
  });
  const proofStatusReport = JSON.parse(proofStatus.result.content[0].text);
  assert.equal(proofStatusReport.runs[0].runId, "mcp-proof-pass");

  const proofClaim = await client.request(16, "tools/call", {
    name: "ocentra_enforcer_proof_claim",
    arguments: {
      root: proofRoot,
      proofId: "PROOF-COMMAND-GENERIC",
      prReady: true,
    },
  });
  assert.equal(proofClaim.result.isError, false);
  assert.equal(JSON.parse(proofClaim.result.content[0].text).ok, true);

  const proofArtifact = await client.request(17, "tools/call", {
    name: "ocentra_enforcer_proof_artifact",
    arguments: {
      root: proofRoot,
      runId: "mcp-proof-pass",
      artifact: "summary",
      limitBytes: 200,
    },
  });
  assert.equal(proofArtifact.result.isError, false);
  assert.match(
    JSON.parse(proofArtifact.result.content[0].text).text,
    /PROOF-COMMAND-GENERIC/u,
  );

  const proofImport = await client.request(18, "tools/call", {
    name: "ocentra_enforcer_proof_import_legacy",
    arguments: {
      root: proofRoot,
      proofId: "PROOF-LEGACY-ARTIFACT-IMPORT",
      legacyPaths: ["test-results/tiny-proof"],
      runId: "mcp-legacy-import",
    },
  });
  assert.equal(proofImport.result.isError, false);
  const proofImportReport = JSON.parse(proofImport.result.content[0].text);
  assert.equal(proofImportReport.proofRun.legacy.artifactCount, 1);

  const proofParity = await client.request(19, "tools/call", {
    name: "ocentra_enforcer_proof_parity",
    arguments: {
      root: proofRoot,
      proofId: "PROOF-LEGACY-ARTIFACT-IMPORT",
      legacyPaths: ["test-results/tiny-proof"],
      runId: "mcp-legacy-import",
    },
  });
  assert.equal(proofParity.result.isError, false);
  const proofParityReport = JSON.parse(proofParity.result.content[0].text);
  assert.equal(proofParityReport.coverage, "equivalent");
  assert.equal(proofParityReport.deletionReady, true);
});

test("MCP status detects stale server code and blocks coordination writes", async (t) => {
  const launcherRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "ocentra-enforcer-mcp-stale-"),
  );
  const watchedFile = path.join(launcherRoot, "watched.txt");
  fs.writeFileSync(watchedFile, "before\n");
  const server = spawn(process.execPath, [SERVER_PATH], {
    cwd: launcherRoot,
    stdio: ["pipe", "pipe", "pipe"],
    env: {
      ...process.env,
      OCENTRA_ENFORCER_MCP_FINGERPRINT_EXTRA: watchedFile,
    },
  });
  t.after(() => {
    server.kill();
  });

  const client = createMcpClient(server);
  const initialized = await client.request(1, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
  });
  assert.equal(initialized.result.serverInfo.name, "ocentra-enforcer");

  const freshStatus = await client.request(2, "tools/call", {
    name: "ocentra_enforcer_mcp_status",
    arguments: {},
  });
  assert.equal(freshStatus.result.isError, false);
  assert.equal(JSON.parse(freshStatus.result.content[0].text).stale, false);

  fs.writeFileSync(watchedFile, "after\n");
  const staleStatus = await client.request(3, "tools/call", {
    name: "ocentra_enforcer_mcp_status",
    arguments: {},
  });
  assert.equal(staleStatus.result.isError, true);
  const staleReport = JSON.parse(staleStatus.result.content[0].text);
  assert.equal(staleReport.stale, true);
  assert.equal(staleReport.reloadRequired, true);
  assert.equal(staleReport.writeCompatible, false);
  assert.equal(staleReport.directWritesAllowed, false);
  assert.equal(staleReport.hashCompatible, true);
  assert.equal(staleReport.changedFiles.length, 1);

  const staleStateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "mcp-stale-ledger-"));
  const staleTargetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "mcp-stale-target-"));
  const staleClaim = await client.request(4, "tools/call", {
    name: "ocentra_enforcer_coordination_claim",
    arguments: {
      stateRoot: staleStateRoot,
      hub: "stale-hub",
      root: staleTargetRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      reason: "must fail closed",
      codexThreadId: "thread-stale",
    },
  });
  assert.equal(staleClaim.result.isError, true);
  assert.match(staleClaim.result.content[0].text, /MCP server is stale/u);
  const staleClaimReport = JSON.parse(staleClaim.result.content[0].text);
  assert.equal(staleClaimReport.reloadRequired, true);
  assert.equal(staleClaimReport.writeCapable, false);
  assert.equal(staleClaimReport.directWritesAllowed, false);
  assert.equal(staleClaimReport.fallbackAvailable, true);
  assert.equal(staleClaimReport.fallback.recommendedTool, "ocentra_enforcer_run");
  assert.equal(staleClaimReport.fallback.cwd, PACK_ROOT);
  assert.deepEqual(staleClaimReport.fallback.command, [
    process.execPath,
    CLI,
    "coordination",
    "claim",
    "--state-root",
    staleStateRoot,
    "--hub",
    "stale-hub",
    "codex-a",
    "src/lib.rs",
    "--root",
    staleTargetRoot,
    "--codex-thread-id",
    "thread-stale",
    "--reason",
    "must fail closed",
    "--json",
  ]);
  assert.deepEqual(
    staleClaimReport.fallback.enforcerRunArguments.command,
    staleClaimReport.fallback.command,
  );
  assert.equal(
    staleClaimReport.fallback.enforcerRunArguments.tool,
    "ocentra-enforcer-coordination-fallback",
  );
  assert.match(staleClaimReport.fallback.commandLine, /coordination claim/u);
  assert.match(staleClaimReport.nextStep, /ocentra_enforcer_run/u);

  const staleMessage = await client.request(5, "tools/call", {
    name: "ocentra_enforcer_coordination_message",
    arguments: {
      stateRoot: staleStateRoot,
      hub: "stale-hub",
      from: "codex-a",
      to: "codex-b",
      subject: "Fallback subject",
      body: "Fallback body.",
    },
  });
  assert.equal(staleMessage.result.isError, true);
  const staleMessageReport = JSON.parse(staleMessage.result.content[0].text);
  assert.deepEqual(staleMessageReport.fallback.command, [
    process.execPath,
    CLI,
    "coordination",
    "message",
    "--state-root",
    staleStateRoot,
    "--hub",
    "stale-hub",
    "--from",
    "codex-a",
    "--to",
    "codex-b",
    "--subject",
    "Fallback subject",
    "--body",
    "Fallback body.",
    "--json",
  ]);
});

test("MCP server supports newline JSON framing and empty Codex probe methods", async (t) => {
  const launcherRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "ocentra-enforcer-mcp-ndjson-"),
  );
  const server = spawn(process.execPath, [SERVER_PATH], {
    cwd: launcherRoot,
    stdio: ["pipe", "pipe", "pipe"],
  });
  t.after(() => {
    server.kill();
  });

  const client = createMcpClient(server, "ndjson");
  const initialized = await client.request(1, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
  });
  assert.equal(initialized.result.serverInfo.name, "ocentra-enforcer");
  const resources = await client.request(2, "resources/list", {});
  assert.deepEqual(resources.result.resources, []);
  const resourceTemplates = await client.request(
    3,
    "resources/templates/list",
    {},
  );
  assert.deepEqual(resourceTemplates.result.resourceTemplates, []);
  const prompts = await client.request(4, "prompts/list", {});
  assert.deepEqual(prompts.result.prompts, []);
  const tools = await client.request(5, "tools/list", {});
  assert.equal(
    tools.result.tools.some((tool) => tool.name === "ocentra_enforcer_route"),
    true,
  );
});

function createMcpClient(server, framing = "content-length") {
  let output = Buffer.alloc(0);
  const received = new Map();
  const waiters = new Map();
  let stderr = "";

  server.stderr.on("data", (chunk) => {
    stderr += chunk.toString("utf8");
  });

  server.stdout.on("data", (chunk) => {
    output = Buffer.concat([output, chunk]);
    while (output.length > 0) {
      const frame = readFrame();
      if (frame === null) return;
      const message = JSON.parse(frame);
      if (message.id !== undefined && waiters.has(message.id)) {
        const waiter = waiters.get(message.id);
        waiters.delete(message.id);
        waiter.resolve(message);
      } else if (message.id !== undefined) {
        received.set(message.id, message);
      }
    }
  });

  return {
    request(id, method, params) {
      server.stdin.write(
        encodeFrame({ jsonrpc: "2.0", id, method, params }, framing),
      );
      return waitFor(id);
    },
    notify(method, params) {
      server.stdin.write(
        encodeFrame({ jsonrpc: "2.0", method, params }, framing),
      );
    },
  };

  function waitFor(id) {
    if (received.has(id)) {
      const message = received.get(id);
      received.delete(id);
      return Promise.resolve(message);
    }
    return new Promise((resolve, reject) => {
      // TIMER-JUSTIFICATION: MCP protocol tests need a bounded child-process response timeout.
      const timeout = setTimeout(() => {
        waiters.delete(id);
        reject(
          new Error(
            `Timed out waiting for MCP response ${id}. stderr=${stderr}`,
          ),
        );
      }, 30000);
      waiters.set(id, {
        resolve(message) {
          clearTimeout(timeout);
          resolve(message);
        },
      });
    });
  }

  function readFrame() {
    if (framing === "ndjson") {
      const lineEnd = output.indexOf("\n");
      if (lineEnd === -1) return null;
      const body = output
        .slice(0, lineEnd)
        .toString("utf8")
        .replace(/\r$/u, "");
      output = output.slice(lineEnd + 1);
      return body;
    }

    const headerEnd = output.indexOf("\r\n\r\n");
    if (headerEnd === -1) return null;
    const header = output.slice(0, headerEnd).toString("utf8");
    const lengthMatch = /content-length:\s*(\d+)/iu.exec(header);
    assert.ok(lengthMatch, `missing Content-Length in ${header}`);
    const contentLength = Number(lengthMatch[1]);
    const start = headerEnd + 4;
    const end = start + contentLength;
    if (output.length < end) return null;
    const body = output.slice(start, end).toString("utf8");
    output = output.slice(end);
    return body;
  }
}

function encodeFrame(message, framing) {
  const body = JSON.stringify(message);
  if (framing === "ndjson") return `${body}\n`;
  return `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`;
}
