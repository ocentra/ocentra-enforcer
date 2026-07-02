#!/usr/bin/env node
/*
 * MCP stdio adapter for Ocentra Enforcer.
 * rust-rules-mcp remains a compatibility filename for one release.
 */
import fs from "node:fs";
import { PACKAGE_JSON } from "./rust-rules-mcp-helpers.mjs";
import { callTool } from "./rust-rules-mcp-dispatch.mjs";
import { TOOLS } from "./rust-rules-mcp-tool-registry.mjs";
import { startMcpStdioServer } from "./rust-rules-mcp-transport.mjs";

startMcpStdioServer({
  callTool,
  packageJson: PACKAGE_JSON,
  tools: TOOLS,
});
