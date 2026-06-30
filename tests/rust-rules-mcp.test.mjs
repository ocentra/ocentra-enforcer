import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const SERVER_PATH = path.join(PACK_ROOT, "mcp", "rust-rules-mcp.mjs");

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
    "ocentra_enforcer_route",
    "ocentra_enforcer_scan",
    "ocentra_enforcer_run",
    "ocentra_enforcer_run_status",
    "ocentra_enforcer_diagnostics",
    "ocentra_enforcer_last_failure",
    "ocentra_enforcer_artifact",
    "ocentra_enforcer_prune_runs",
    "ocentra_enforcer_reset_runs",
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
        'console.error("src/app.ts(2,1): error TS1005: ; expected."); process.exit(1);',
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
