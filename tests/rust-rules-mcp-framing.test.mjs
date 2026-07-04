import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { createMcpClient } from "./mcp-client.mjs";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const SERVER_PATH = path.join(PACK_ROOT, "mcp", "ocentra-enforcer-mcp.mjs");

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
