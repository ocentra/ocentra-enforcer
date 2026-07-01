#!/usr/bin/env node
/*
 * Canonical Ocentra Enforcer MCP entrypoint.
 * rust-rules-mcp remains a compatibility alias for one release.
 */
import { spawnSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const mcpPath = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "rust-rules-mcp.mjs",
);

const result = spawnSync(process.execPath, [mcpPath, ...process.argv.slice(2)], {
  stdio: "inherit",
  shell: false,
});

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 1);
